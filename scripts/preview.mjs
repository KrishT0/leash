// Build a stand-alone preview of the UI.
//
// The frontend talks to the engine through Tauri's invoke. In a plain browser
// there is no engine, so this writes dist/preview.html: the real built bundle,
// with the Tauri bridge stubbed and a set of realistic readings in its place.
//
// It exists so the pages can be looked at, and screenshotted, without starting
// the desktop app. The data below is the same data the design mockups use, so a
// screenshot of the app and a screenshot of a mockup can be compared directly.
//
//   node scripts/preview.mjs            then open dist/preview.html
//   node scripts/preview.mjs#services   the hash clicks that page for you

import { readFileSync, writeFileSync } from "node:fs";

const here = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const index = readFileSync(`${here}dist/index.html`, "utf8");

const script = index.match(/<script[^>]+src="([^"]+)"/)?.[1];
const style = index.match(/<link[^>]+href="([^"]+\.css)"/)?.[1];
if (!script) {
  throw new Error("no bundle in dist/index.html. Run pnpm build first.");
}

const service = (part) => ({
  name: "Spooler",
  display_name: "Print Spooler",
  description: "",
  status: "stopped",
  startup: "automatic",
  accepts_stop: true,
  is_driver: false,
  is_win32: true,
  third_party: false,
  pid: null,
  verdict: { level: "normal", reason: "none" },
  verdict_text: "",
  policy: "stop",
  gave_up: false,
  dependents: [],
  suggested: null,
  ...part,
});

const services = [
  service({
    display_name: "Print Spooler",
    description: "Loads files to memory for later printing.",
    suggested: {
      name: "Spooler",
      display: "Print Spooler",
      reason: "Only needed if you print",
      effect: "You cannot print",
      risk: "medium",
    },
  }),
  service({
    name: "WSearch",
    display_name: "Windows Search",
    status: "running",
    pid: 4212,
  }),
  service({
    name: "SysMain",
    display_name: "SysMain",
    status: "running",
    pid: 1180,
    policy: null,
    suggested: {
      name: "SysMain",
      display: "SysMain",
      reason: "Not needed on a solid state drive",
      effect: "Apps may open a little slower",
      risk: "low",
    },
  }),
  service({
    name: "wuauserv",
    display_name: "Windows Update",
    status: "stopped",
    startup: "manual",
    policy: null,
    verdict: { level: "critical", reason: "sensitive_list" },
    verdict_text: "it is a critical service, so Leash warns first",
    dependents: ["UsoSvc", "DoSvc"],
  }),
  service({
    name: "RpcSs",
    display_name: "Remote Procedure Call",
    status: "running",
    pid: 936,
    policy: null,
    verdict: { level: "blocked", reason: "deny_list" },
    verdict_text: "it is on the list of services Leash never stops",
  }),
  service({
    name: "DiagTrack",
    display_name: "Connected User Experiences and Telemetry",
    status: "running",
    pid: 2684,
    policy: "notify",
    suggested: {
      name: "DiagTrack",
      display: "Connected User Experiences and Telemetry",
      reason: "Sends usage data to Microsoft",
      effect: "Diagnostic data stops being sent",
      risk: "medium",
    },
  }),
  service({
    name: "bthserv",
    display_name: "Bluetooth Support Service",
    status: "running",
    pid: 3312,
    policy: null,
  }),
  service({ name: "Fax", display_name: "Fax", startup: "manual", policy: null }),
];

const snapshot = {
  elevated: true,
  paused: false,
  watching: true,
  monitored: services.filter((row) => row.policy !== null),
  unwatched: [],
  caught_today: 23,
  weekly: [
    { date: "2026-10-05", count: 14 },
    { date: "2026-10-06", count: 22 },
    { date: "2026-10-07", count: 9 },
    { date: "2026-10-08", count: 31 },
    { date: "2026-10-09", count: 18 },
    { date: "2026-10-10", count: 26 },
    { date: "2026-10-11", count: 23 },
  ],
  last_catch: new Date(Date.now() - 462_000).toISOString(),
  general: {
    paused: false,
    notifications: true,
    start_with_windows: true,
    start_minimized: true,
    start_watchdog: true,
  },
  config_error: null,
};

const line = (minutes, service, action, result, error = null) => ({
  ts: new Date(Date.now() - minutes * 60_000).toISOString(),
  service,
  event: action === "stop" && result === "success" ? "started" : "started",
  action,
  result,
  error,
});

const log = [
  line(3, "edgeupdate", "stop", "success"),
  line(6, "Spooler", "stop", "success"),
  line(33, "XboxGipSvc", null, "blocked"),
  line(47, "DiagTrack", "notify", "notified"),
  line(64, "bthserv", "dry_run", "would_stop"),
  { ts: new Date(Date.now() - 75 * 60_000).toISOString(), service: "Spooler", event: "manual", action: "stop", result: "success", error: null },
  line(93, "wuauserv", null, "not_elevated"),
  line(107, "SysMain", "stop", "success"),
  line(125, "RpcSs", null, "blocked"),
  line(134, "Spooler", "stop", "failure", "the service did not stop in time"),
];

const stub = `
window.__TAURI_INTERNALS__ = {
  transformCallback: function (callback) {
    var id = Math.floor(Math.random() * 100000);
    window["_" + id] = callback;
    return id;
  },
  invoke: async function (command, args) {
    if (command === "get_state") return ${JSON.stringify(snapshot)};
    if (command === "list_services") return ${JSON.stringify(services)};
    if (command === "get_log") return ${JSON.stringify(log)};
    if (command === "get_service") {
      var all = ${JSON.stringify(services)};
      return all.find(function (row) { return row.name === (args && args.name); }) || all[0];
    }
    if (String(command).indexOf("plugin:") === 0) return 1;
    return null;
  },
};
`;

const clicker = `
var wanted = location.hash.slice(1) || "home";
var names = { home: "Home", services: "Services", activity: "Activity", settings: "Settings" };
var withSheet = wanted.indexOf("-sheet") > 0;
var label = names[wanted.replace("-sheet", "")];
if (label) {
  var tries = 0;
  var timer = setInterval(function () {
    var button = document.querySelector('[aria-label="' + label + '"]');
    if (button) {
      button.click();
      clearInterval(timer);
      if (withSheet) {
        setTimeout(function () {
          var wantedName = "${services[0].display_name}";
          var row = [].slice.call(document.querySelectorAll("button")).filter(function (button) {
            return button.textContent.indexOf(wantedName) >= 0;
          })[0];
          if (row) row.click();
        }, 400);
      }
    }
    if (++tries > 40) clearInterval(timer);
  }, 50);
}
`;

const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Leash preview</title>
${style ? `<link rel="stylesheet" href="${style.replace(/^\//, "")}">` : ""}
<script>${stub}</script>
</head>
<body>
<div id="root"></div>
<script>${clicker}</script>
<script type="module" src="${script.replace(/^\//, "")}"></script>
</body>
</html>
`;

writeFileSync(`${here}dist/preview.html`, html);
console.log(`wrote dist/preview.html (bundle ${script})`);

// A browser refuses to load an ES module over file://, so the preview is served
// over http instead. Run with --serve and open the printed address.
if (process.argv.includes("--serve")) {
  const { createServer } = await import("node:http");
  const { readFileSync: read, existsSync } = await import("node:fs");
  const types = { html: "text/html", js: "text/javascript", css: "text/css", woff2: "font/woff2" };
  const port = 4599;

  createServer((request, response) => {
    const path = `${here}dist/${decodeURIComponent(request.url.split("?")[0].split("#")[0]).replace(/^\//, "")}`;
    if (!existsSync(path)) {
      response.writeHead(404).end("not found");
      return;
    }
    const extension = path.split(".").pop();
    response.writeHead(200, { "content-type": types[extension] ?? "application/octet-stream" });
    response.end(read(path));
  }).listen(port, "127.0.0.1", () => {
    console.log(`serving http://127.0.0.1:${port}/preview.html`);
  });
}
