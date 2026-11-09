import { spawn } from "node:child_process";
const child = spawn("C:/Users/Pranshul/Documents/Projects/protonsearch/dist-installer/ProtonSearch.exe", [], {
  detached: true,
  stdio: "ignore",
  env: { ...process.env,
    PROTONSEARCH_DATA_DIR: "C:/Users/Pranshul/Documents/Projects/protonsearch/docs/qa/g8-verify-round2/appdata",
    PROTONSEARCH_CDP_PORT: "9223",
    WEBVIEW2_USER_DATA_FOLDER: "C:/Users/Pranshul/Documents/Projects/protonsearch/docs/qa/g8-verify-round2/webview2" }
});
child.unref();
console.log("SPAWNED_PID=" + child.pid);
setTimeout(() => process.exit(0), 500);
