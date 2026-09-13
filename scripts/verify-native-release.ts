import path from "node:path";
import { verifyNativeRelease } from "./native-release";

if (!process.argv[2]) throw new Error("Provide the native release directory");
verifyNativeRelease(path.resolve(process.argv[2])).then(
  (result) => console.log(JSON.stringify(result, null, 2)),
  (error: unknown) => {
    console.error(error);
    process.exitCode = 1;
  },
);
