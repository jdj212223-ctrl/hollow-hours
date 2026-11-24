# Kind Radio: an example Python mod for Hollow Hours (4.0).
# Put this folder (or a .hh zip of it) in Hollow Hours' mods/local folder.
# Your functions get `game` (a dict: place, infection, food, water, fear, survived, credits, night, hidden, near,
# infected, time) and return a list of actions: say(who, text), log(text), give(food=, water=), credits(n),
# fear(x), item("food" | "water"). Scripts can't open files, use the network or import other modules.

checked = {"food": False}

def on_start(game):
    return [say("Ember", "Radio check. Kind Radio is on, we'll keep an eye on you.")]

def on_tick(game):
    out = []
    if game["food"] < 0.2 and not checked["food"]:
        checked["food"] = True
        out += [say("Clove", "You're starving. I left something for you nearby."), item("food")]
    if game["food"] > 0.5:
        checked["food"] = False
    if game["infection"] > 0.6 and random.random() < 0.2:
        out.append(log("(radio) Atlas: Your breathing sounds wrong. Find an antidote. Please."))
    if game["hidden"] and random.random() < 0.1:
        out.append(log("(radio) Kigo: good spot. / stay. / they're passing."))
    return out

def on_night(game, n):
    return [log(f"(radio) Verse: Night {n}. We're all on this channel. Talk to us.")]

def on_over(game, why):
    return [log("(radio) Ember: ...We'll find you again. We always do.")]
