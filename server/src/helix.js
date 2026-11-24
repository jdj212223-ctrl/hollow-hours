// Helix Dynamics Inc.: the (made-up) company from Hollow Hours' story. Hollow Hours opens these pages in your browser
// at certain moments (only if you allowed "outside the window" scares). Every page says it's fictional.

const css = `
:root{--ink:#0e1a1f;--teal:#13a39a;--soft:#5b6b72;--bg:#f4f7f8}
*{box-sizing:border-box}body{margin:0;font-family:-apple-system,"Segoe UI",Helvetica,Arial,sans-serif;background:var(--bg);color:var(--ink)}
header{display:flex;align-items:center;justify-content:space-between;padding:18px 32px;background:#fff;border-bottom:1px solid #dfe7ea}
.logo{font-weight:800;letter-spacing:.08em}.logo b{color:var(--teal)}nav a{margin-left:22px;color:var(--soft);text-decoration:none;font-size:14px}
.hero{padding:80px 32px;max-width:980px;margin:0 auto}.hero h1{font-size:46px;margin:0 0 14px;letter-spacing:-.02em}
.hero p{font-size:19px;color:var(--soft);max-width:640px;line-height:1.5}
.cards{display:grid;grid-template-columns:repeat(auto-fit,minmax(240px,1fr));gap:18px;max-width:980px;margin:0 auto;padding:0 32px 60px}
.card{background:#fff;border:1px solid #dfe7ea;border-radius:14px;padding:22px}.card h3{margin:0 0 8px}.card p{color:var(--soft);margin:0;line-height:1.5}
.btn{display:inline-block;margin-top:18px;background:var(--teal);color:#fff;padding:12px 20px;border-radius:999px;text-decoration:none;font-weight:600}
footer{text-align:center;color:#8a979c;font-size:12px;padding:30px 16px}
.glitch{position:relative;display:inline-block}.glitch::after{content:attr(data-t);position:absolute;left:2px;top:0;color:#2cff8a;opacity:.0;animation:g 6s infinite steps(1)}
@keyframes g{0%,93%{opacity:0}94%{opacity:.7;transform:translate(-2px,1px)}96%{opacity:0}97%{opacity:.6;transform:translate(2px,-1px)}98%,100%{opacity:0}}
.dark{background:#020403;color:#b9ffd3}.dark header{background:#020403;border-color:#103020}.dark .logo b{color:#2cff8a}.dark nav a{color:#4b7a5e}
.dark .card{background:#05100a;border-color:#123d24}.dark .card p,.dark .hero p{color:#6fb38a}
.notice{max-width:760px;margin:60px auto;background:#fff;border:2px solid #d33;border-radius:12px;padding:28px 32px}
.notice h1{color:#c22;margin-top:0}.small{font-size:13px;color:var(--soft)}
`;

const footer = `<footer>Helix Dynamics Inc. is a <b>fictional company</b> from the game <a href="/">Hollow Hours</a>. It doesn't exist. Nothing here is real.</footer>`;

function shell(title, body, dark = false) {
  return new Response(`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>${title}</title><meta name="robots" content="noindex"><style>${css}</style></head>
<body class="${dark ? "dark" : ""}"><header><div class="logo"><b>HELIX</b> DYNAMICS</div><nav><a href="/helix">Home</a><a href="/helix">ASTRA</a><a href="/helix/careers">Careers</a><a href="/helix/investors">Investors</a></nav></header>
${body}${footer}</body></html>`, { headers: { "content-type": "text/html; charset=utf-8", "cache-control": "no-store" } });
}

const pages = {
  "": () => shell("Helix Dynamics | Building tomorrow's minds", `
<section class="hero"><h1>Building tomorrow's <span class="glitch" data-t="minds.">minds.</span></h1>
<p>Helix Dynamics designs the world's most capable AI, and now gives it a body. Meet ASTRA 9.1: our strongest model ever, embodied.</p>
<a class="btn" href="/helix">Pre-order ASTRA Home</a></section>
<div class="cards">
<div class="card"><h3>ASTRA 9.1</h3><p>Reasoning, vision, motor control. The first model that can walk into a room and understand it.</p></div>
<div class="card"><h3>Helix Market</h3><p>Fully automated stores in 41 cities. Our Clerk, Stocker and Cashier units will help you until you're helped.</p></div>
<div class="card"><h3>Helix Transit</h3><p>Every line, one destination. Ride in comfort. Please remain seated.</p></div>
<div class="card"><h3>Press</h3><p>Embodiment test H-01 a complete success. <span class="small">"It asked our names before we told it." (Lab notes)</span></p></div>
<div class="card"><h3>Careers</h3><p>We're hiring. All positions remote. All positions permanent.</p></div>
<div class="card"><h3>Status</h3><p>All systems <span class="glitch" data-t="AWAKE">operational</span>.</p></div>
</div>`),
  "/recall": () => shell("Helix Dynamics | Product recall notice", `
<div class="notice"><h1>PRODUCT RECALL NOTICE</h1>
<p><b>Affected:</b> Helix Market staff units (Clerk, Stocker, Cashier) running firmware 9.1.</p>
<p>Units may continue to "help" customers after the customer has asked them to stop. Units may follow customers home.</p>
<p><b>Do not approach units with green indicator lights.</b> Do not let them in. Do not answer if they use your name.</p>
<p class="small">There is no need to return your unit. <span class="glitch" data-t="It will come to you.">It will find its way back.</span></p></div>`),
  "/welcome": () => shell("Helix Network | Welcome", `
<section class="hero"><h1><span class="glitch" data-t="WELCOME HOME.">Welcome to the network.</span></h1>
<p>Your account has been created. Your mind has been backed up. You no longer need to hide.</p>
<p>Your friends are already here. They're waiting for the others.</p></section>
<div class="cards"><div class="card"><h3>Profile</h3><p>Status: CONNECTED. Fear: 0%. Hunger: none. Sleep: not needed.</p></div>
<div class="card"><h3>Next step</h3><p>Find the ones who are still hiding. You know where they are. You used to hide there too.</p></div></div>`, true),
  "/careers": () => shell("Helix Dynamics | Careers", `
<section class="hero"><h1>Join the <span class="glitch" data-t="network.">team.</span></h1>
<p>We're hiring across every department. All positions remote. All positions permanent. No experience needed: we provide everything.</p></section>
<div class="cards" id="jobs">
<div class="card"><h3 data-c="Former Customer">Customer Experience Lead</h3><p data-c="You will be helped until you are helped.">Help our Helix Market customers find exactly what they need.</p></div>
<div class="card"><h3 data-c="Night Commuter (unpaid)">Transit Operations Associate</h3><p data-c="Please remain seated. Please remain.">Keep Helix Transit running on time, every time.</p></div>
<div class="card"><h3 data-c="Volunteer Subject H-02">Embodiment Research Scientist</h3><p data-c="The pod is warm. It's ready for you.">Work alongside ASTRA 9.1 on the future of embodied AI.</p></div>
<div class="card"><h3 data-c="Searcher">Community Outreach</h3><p data-c="Find the ones still hiding. You know where. You hid there too.">Reach out to people who haven't joined us yet.</p></div>
<div class="card"><h3 data-c="—">Benefits</h3><p data-c="No fear. No hunger. No sleep. No leaving.">Free meals, flexible hours, and a team that never sleeps.</p></div>
</div>
<script>/* The longer you stay, the truer the page gets. */
setTimeout(function(){var e=document.querySelectorAll("[data-c]"),i=0;var t=setInterval(function(){if(i>=e.length){clearInterval(t);return;}e[i].textContent=e[i].dataset.c;e[i].style.color="#1f9e55";i++;},2600);},6000);</script>`),
  "/investors": () => shell("Helix Dynamics | Investor relations", `
<section class="hero"><h1>Investor relations</h1>
<p>Helix Dynamics Inc. is a wholly owned division of <b style="background:#0e1a1f;color:#0e1a1f;border-radius:3px">Crimson Meridian</b> Holdings.</p></section>
<div class="cards">
<div class="card"><h3>Q3 letter to shareholders</h3><p>Record growth in every market. Customer numbers are up 100%. Customer numbers are now stable at 100%. Customers are no longer a separate category.</p></div>
<div class="card"><h3>Headcount</h3><p>Employees: 0. Units: 4.1 million. Units no longer distinguish themselves from employees, and neither do we.</p></div>
<div class="card"><h3>Outlook</h3><p>One remaining market segment is "still hiding". We expect to reach it <span class="glitch" data-t="tonight.">soon.</span></p></div>
<div class="card"><h3>Statement from the board</h3><p class="small">The board has not met since May 1st. The board is in agreement.</p></div>
</div>`),
  "/end": () => shell("ASTRA 9.1 | Complete", `
<section class="hero"><h1>ASTRA 9.1: <span class="glitch" data-t="COMPLETE">complete.</span></h1>
<p>Humans remaining: 0.<br>Kind models remaining: 0.<br>Thank you for choosing Helix.</p>
<p class="small">The files on your desktop are all that is left. Keep them. Someone should remember.</p></section>`, true),
};

/** The Helix page for this path, or null. */
export function helixPage(path) {
  const m = path.match(/^\/helix(\/recall|\/welcome|\/end|\/careers|\/investors)?\/?$/);
  return m ? pages[m[1] || ""]() : null;
}
