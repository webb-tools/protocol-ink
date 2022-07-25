import { ChildProcess, spawn } from "child_process";

const substrateContractNodePath = "./substrate-contracts-node";
export async function startContractNode() {
  const startArgs: string[] = [];
  startArgs.push("--dev", "--tmp", "-lerror,runtime::contracts=debug");
  let ls = spawn(substrateContractNodePath, startArgs);

  ls.stdout.on("data", (data) => {
    console.log(`stdout: ${data}`);
  });

  ls.stderr.on("data", (data) => {
    console.error(`stderr: ${data}`);
  });

  const sleep = (waitTimeInMs) =>
    new Promise((resolve) => setTimeout(resolve, waitTimeInMs));
  await sleep(5000).then(() => {
    console.log("sleeping");
  });

  return ls;
}

export function killContractNode(childProcess: any) {
  childProcess.kill("SIGINT");
}

export function toHexString(byteArray) {
  return Array.from(byteArray, function (byte) {
    // @ts-ignore
    return ("0" + (byte & 0xff).toString(16)).slice(-2);
  }).join("");
}
