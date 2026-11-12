import { spawn } from "node:child_process";
const child = spawn("C:/Users/Pranshul/Documents/Projects/protonsearch/target/debug/ProtonSearch.exe", [], {
  detached: true, stdio: "ignore",
  env: { ...process.env,
    PROTONSEARCH_DATA_DIR: "C:/Users/Pranshul/Documents/Projects/protonsearch/docs/qa/g8-verify-round3/appdata",
    PROTONSEARCH_CDP_PORT: "9223",
    WEBVIEW2_USER_DATA_FOLDER: "C:/Users/Pranshul/Documents/Projects/protonsearch/docs/qa/g8-verify-round3/webview2" }
});
child.unref(); console.log("PID=" + child.pid); setTimeout(() => process.exit(0), 500);
