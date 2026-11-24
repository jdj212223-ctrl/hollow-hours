package io.github.hollowhours.hhlink;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import net.minecraft.ChatFormatting;
import net.minecraft.core.BlockPos;
import net.minecraft.core.component.DataComponents;
import net.minecraft.network.chat.Component;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerBossEvent;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.sounds.SoundEvents;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.BossEvent;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.MobSpawnType;
import net.minecraft.world.entity.monster.Zombie;
import net.minecraft.world.item.Items;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.entity.SignBlockEntity;
import net.minecraft.world.level.block.entity.SignText;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.level.entity.EntityTypeTest;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.bus.api.SubscribeEvent;
import net.neoforged.fml.common.Mod;
import net.neoforged.neoforge.common.NeoForge;
import net.neoforged.neoforge.event.entity.living.LivingDeathEvent;
import net.neoforged.neoforge.event.entity.living.LivingEntityUseItemEvent;
import net.neoforged.neoforge.event.entity.player.PlayerEvent;
import net.neoforged.neoforge.event.entity.player.PlayerWakeUpEvent;
import net.neoforged.neoforge.event.tick.ServerTickEvent;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.Random;

/**
 * HH-Link: Minecraft, linked to Hollow Hours. Once a second it asks the running game (its local bridge on
 * 127.0.0.1, locked with the same token the game keeps in its own folder) what's happening, and shows it here.
 * Things you do here go back (eating, sleeping, dying). It never talks to anything but your own computer.
 */
@Mod(HHLink.ID)
public class HHLink {
    public static final String ID = "hhlink";
    static final Logger LOG = LoggerFactory.getLogger("HH-Link");
    static final String TAG = "hh_link";

    private final HttpClient http = HttpClient.newBuilder().connectTimeout(Duration.ofMillis(800)).build();
    private volatile JsonObject state;
    private volatile boolean asking;
    private boolean linked, wasTornado, wasBreach, signed;
    private int ticks;
    private final Random rnd = new Random();
    private ServerBossEvent bar;

    public HHLink(IEventBus modBus) {
        NeoForge.EVENT_BUS.register(this);
        LOG.info("HH-Link loaded: waiting for Hollow Hours on this computer.");
    }

    // ---------------- talking to the game ----------------

    /** Hollow Hours' folder (the same place the game keeps bridge.json). */
    static Path dataDir() {
        String over = System.getenv("HOLLOW_HOURS_DIR"); // for testing with a second copy of the game
        if (over != null) return Path.of(over);
        String os = System.getProperty("os.name").toLowerCase();
        Path home = Path.of(System.getProperty("user.home"));
        if (os.contains("mac")) return home.resolve("Library/Application Support/Hollow Hours");
        if (os.contains("win")) {
            String app = System.getenv("APPDATA");
            return (app != null ? Path.of(app) : home.resolve("AppData/Roaming")).resolve("Hollow Hours");
        }
        String xdg = System.getenv("XDG_DATA_HOME");
        return (xdg != null ? Path.of(xdg) : home.resolve(".local/share")).resolve("hollow-hours");
    }

    private void send(String cmd, JsonObject args, boolean keep) {
        try {
            JsonObject b = JsonParser.parseString(Files.readString(dataDir().resolve("bridge.json"))).getAsJsonObject();
            JsonObject body = new JsonObject();
            body.addProperty("cmd", cmd);
            body.add("args", args == null ? new JsonObject() : args);
            HttpRequest req = HttpRequest.newBuilder(URI.create("http://127.0.0.1:" + b.get("port").getAsInt() + "/"))
                    .timeout(Duration.ofSeconds(2))
                    .header("Authorization", "Bearer " + b.get("token").getAsString())
                    .POST(HttpRequest.BodyPublishers.ofString(body.toString())).build();
            if (keep) asking = true;
            http.sendAsync(req, HttpResponse.BodyHandlers.ofString()).whenComplete((r, err) -> {
                if (keep) {
                    asking = false;
                    if (err == null && r.statusCode() == 200) {
                        try { state = JsonParser.parseString(r.body()).getAsJsonObject(); } catch (Exception e) { state = null; }
                    } else state = null;
                }
            });
        } catch (Exception e) {
            if (keep) state = null; // the game isn't running (no bridge.json, or nobody listening)
        }
    }

    private void event(String name) {
        if (!linked) return;
        JsonObject a = new JsonObject();
        a.addProperty("event", name);
        send("mc_event", a, false);
    }

    // ---------------- every tick ----------------

    @SubscribeEvent
    public void onTick(ServerTickEvent.Post e) {
        MinecraftServer server = e.getServer();
        if (++ticks % 20 == 0 && !asking) send("mc_state", null, true);
        if (ticks % 20 != 1) return;
        JsonObject s = state;
        boolean now = s != null && s.has("playing");
        if (now != linked) {
            linked = now;
            LOG.info(linked ? "HH-Link: linked to Hollow Hours." : "HH-Link: Hollow Hours isn't answering.");
            if (linked) for (ServerPlayer p : server.getPlayerList().getPlayers())
                p.sendSystemMessage(Component.literal("[Hollow Hours] linked. something on the other side is listening.").withStyle(ChatFormatting.DARK_GREEN, ChatFormatting.ITALIC));
            if (linked) event("hello");
        }
        boolean playing = linked && s.get("playing").getAsBoolean();
        updateBar(server, s, playing);
        if (!playing) { clearInfected(server); return; }

        for (JsonElement w : arr(s, "whispers")) {
            JsonObject o = w.getAsJsonObject();
            Component line = Component.literal(o.get("who").getAsString() + ": " + o.get("line").getAsString()).withStyle(ChatFormatting.GREEN, ChatFormatting.ITALIC);
            for (ServerPlayer p : server.getPlayerList().getPlayers()) p.displayClientMessage(line, true);
        }
        boolean tornado = s.get("tornado").getAsBoolean();
        if (tornado && !wasTornado) for (ServerLevel l : server.getAllLevels()) l.setWeatherParameters(0, 1200, true, true);
        wasTornado = tornado;
        boolean breach = s.get("breach").getAsBoolean();
        if (breach && !wasBreach) for (ServerPlayer p : server.getPlayerList().getPlayers())
            p.serverLevel().playSound(null, p.blockPosition(), SoundEvents.ZOMBIE_BREAK_WOODEN_DOOR, SoundSource.HOSTILE, 1.0f, 0.8f);
        wasBreach = breach;
        syncInfected(server, arr(s, "infected_near"));
        if (s.has("build") && s.get("build").getAsBoolean() && ticks % 300 == 1) build(server);
    }

    private static JsonArray arr(JsonObject s, String k) { return s.has(k) && s.get(k).isJsonArray() ? s.getAsJsonArray(k) : new JsonArray(); }

    private void updateBar(MinecraftServer server, JsonObject s, boolean playing) {
        if (bar == null) {
            bar = new ServerBossEvent(Component.literal("HOLLOW HOURS"), BossEvent.BossBarColor.GREEN, BossEvent.BossBarOverlay.NOTCHED_10);
        }
        for (ServerPlayer p : server.getPlayerList().getPlayers()) if (!bar.getPlayers().contains(p)) bar.addPlayer(p);
        bar.setVisible(playing);
        if (!playing) return;
        float inf = s.get("infection").getAsFloat();
        String night = s.has("night") && !s.get("night").isJsonNull() ? " · NIGHT " + s.get("night").getAsInt() : "";
        bar.setProgress(Math.max(0f, Math.min(1f, inf)));
        bar.setName(Component.literal("HOLLOW HOURS · infection " + Math.round(inf * 100) + "% · " + s.get("place").getAsString() + night));
    }

    // ---------------- the infected, over here ----------------

    private List<Zombie> ours(ServerLevel l) {
        return new ArrayList<>(l.getEntities(EntityTypeTest.forClass(Zombie.class), z -> z.getTags().contains(TAG)));
    }

    private void clearInfected(MinecraftServer server) {
        for (ServerLevel l : server.getAllLevels()) for (Zombie z : ours(l)) z.discard();
    }

    /** The infected near you in Hollow Hours stand around you here too (green eyes, their names). */
    private void syncInfected(MinecraftServer server, JsonArray near) {
        List<String> names = new ArrayList<>();
        for (JsonElement n : near) names.add(n.getAsString());
        for (ServerPlayer p : server.getPlayerList().getPlayers()) {
            ServerLevel l = p.serverLevel();
            List<Zombie> have = ours(l);
            for (Zombie z : have) {
                String n = z.getCustomName() == null ? "" : z.getCustomName().getString().replace("Infected ", "");
                if (!names.contains(n)) z.discard();
            }
            for (String n : names) {
                boolean there = have.stream().anyMatch(z -> z.isAlive() && z.getCustomName() != null && z.getCustomName().getString().equals("Infected " + n));
                if (there) continue;
                Zombie z = EntityType.ZOMBIE.create(l);
                if (z == null) continue;
                double a = rnd.nextDouble() * Math.PI * 2, d = 10 + rnd.nextDouble() * 6;
                int x = (int) (p.getX() + Math.cos(a) * d), zz = (int) (p.getZ() + Math.sin(a) * d);
                int y = l.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES, x, zz);
                z.moveTo(x + 0.5, y, zz + 0.5, rnd.nextFloat() * 360f, 0f);
                z.finalizeSpawn(l, l.getCurrentDifficultyAt(z.blockPosition()), MobSpawnType.EVENT, null);
                z.setCustomName(Component.literal("Infected " + n).withStyle(ChatFormatting.GREEN));
                z.setCustomNameVisible(true);
                z.setGlowingTag(true);
                z.addTag(TAG);
                l.addFreshEntity(z);
            }
        }
    }

    /** With Deep meta and your Yes in the game: green moss creeps around you, and once, a sign. */
    private void build(MinecraftServer server) {
        for (ServerPlayer p : server.getPlayerList().getPlayers()) {
            ServerLevel l = p.serverLevel();
            for (int i = 0; i < 6; i++) {
                int x = p.getBlockX() + rnd.nextInt(21) - 10, z = p.getBlockZ() + rnd.nextInt(21) - 10;
                BlockPos at = new BlockPos(x, l.getHeight(Heightmap.Types.WORLD_SURFACE, x, z) - 1, z);
                if (l.getBlockState(at).is(Blocks.GRASS_BLOCK)) l.setBlockAndUpdate(at, Blocks.MOSS_BLOCK.defaultBlockState());
            }
            if (!signed) {
                int x = p.getBlockX() + 3, z = p.getBlockZ() + 3;
                BlockPos at = new BlockPos(x, l.getHeight(Heightmap.Types.MOTION_BLOCKING, x, z), z);
                if (l.getBlockState(at).isAir()) {
                    l.setBlockAndUpdate(at, Blocks.OAK_SIGN.defaultBlockState());
                    if (l.getBlockEntity(at) instanceof SignBlockEntity sign) {
                        SignText t = new SignText().setMessage(0, Component.literal("come to the")).setMessage(1, Component.literal("beach."))
                                .setMessage(2, Component.literal("3:00 AM")).setColor(net.minecraft.world.item.DyeColor.LIME);
                        sign.setText(t, true);
                        sign.setChanged();
                    }
                    signed = true;
                }
            }
        }
    }

    // ---------------- over here → over there ----------------

    @SubscribeEvent
    public void onEat(LivingEntityUseItemEvent.Finish e) {
        if (!(e.getEntity() instanceof ServerPlayer)) return;
        if (e.getItem().is(Items.POTION) || e.getItem().is(Items.MILK_BUCKET)) event("drank");
        else if (e.getItem().has(DataComponents.FOOD)) event("ate");
    }

    @SubscribeEvent
    public void onDeath(LivingDeathEvent e) { if (e.getEntity() instanceof ServerPlayer) event("died"); }

    @SubscribeEvent
    public void onWake(PlayerWakeUpEvent e) { if (e.getEntity() instanceof ServerPlayer) event("slept"); }

    @SubscribeEvent
    public void onLogin(PlayerEvent.PlayerLoggedInEvent e) { if (linked) event("hello"); }
}
