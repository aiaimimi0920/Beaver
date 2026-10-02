import { useState } from "react";
import { call } from "./api";

type Request = {
  source: string;
  sourceProjectId: string;
  targetProjectId: string;
  requestId: string;
};
type Preparation = { request: Request; entities: number; calls: number };
type Assembly = {
  activated: boolean;
  assembly: {
    projectId: string;
    binding: string;
    preparationSha256: string;
    tasksInterrupted: number;
    sessionPathsRewritten: number;
  };
};
type Registration = {
  projectId: string;
  registrationCommitted: boolean;
  runtimeReady: boolean;
  runtimeError?: string;
  schedulerWakeError?: string;
};
export type DerivationMode = "new" | "prepared" | "registered";

export function useProjectDerivation() {
  const [mode, setMode] = useState<DerivationMode>("new");
  const [source, setSource] = useState("");
  const [preparation, setPreparation] = useState("");
  const [destination, setDestination] = useState("");
  const [inspected, setInspected] = useState<Preparation>();
  const [prepared, setPrepared] = useState<Preparation>();
  const [assembly, setAssembly] = useState<Assembly>();
  const [registration, setRegistration] = useState<Registration>();
  const [registrationAttempted, setRegistrationAttempted] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [failedPreparation, setFailedPreparation] = useState("");
  const [failedAssembly, setFailedAssembly] = useState("");
  const paths = () => ({
    preparation: preparation.trim(),
    destination: destination.trim(),
  });
  function resetAssembly() {
    setAssembly(undefined);
    setRegistration(undefined);
    setRegistrationAttempted(false);
    setConfirm(false);
  }
  function resetPreparation() {
    setPrepared(undefined);
    resetAssembly();
  }
  return {
    mode,
    source,
    preparation,
    destination,
    inspected,
    prepared,
    assembly,
    registration,
    registrationAttempted,
    confirm,
    failedPreparation,
    failedAssembly,
    setConfirm,
    changeMode(value: DerivationMode) {
      setMode(value);
      setInspected(undefined);
      resetPreparation();
    },
    changeSource(value: string) {
      setSource(value);
      setInspected(undefined);
      resetPreparation();
    },
    changePreparation(value: string) {
      setPreparation(value);
      resetPreparation();
    },
    changeDestination(value: string) {
      setDestination(value);
      resetAssembly();
    },
    async inspectSource() {
      setInspected(undefined);
      resetPreparation();
      const result = await call<Preparation>(
        "migration.inspectDerivationSource",
        {
          source: source.trim(),
        },
      );
      setSource(result.request.source);
      setInspected(result);
    },
    async prepare() {
      if (!inspected) return;
      try {
        setPrepared(
          await call<Preparation>("migration.prepareDerivation", {
            ...inspected.request,
            preparation: preparation.trim(),
          }),
        );
      } catch (error) {
        setFailedPreparation(preparation.trim());
        throw error;
      }
    },
    async inspectPreparation() {
      resetPreparation();
      setPrepared(
        await call<Preparation>("migration.inspectDerivation", {
          preparation: preparation.trim(),
        }),
      );
    },
    async assemble() {
      resetAssembly();
      try {
        setAssembly(
          await call<Assembly>("migration.assembleDerivation", paths()),
        );
      } catch (error) {
        setFailedAssembly(destination.trim());
        throw error;
      }
    },
    async inspectAssembly() {
      resetAssembly();
      setAssembly(await call<Assembly>("migration.inspectAssembly", paths()));
    },
    async activate() {
      if (!assembly || !confirm) return;
      await call("migration.activateAssembly", paths());
      setAssembly({ ...assembly, activated: true });
      setConfirm(false);
    },
    async register() {
      setRegistrationAttempted(true);
      const result = await call<Registration>(
        "migration.registerAssembly",
        paths(),
      );
      if (!result.registrationCommitted || !result.projectId)
        throw new Error("登记回执异常，请保留目录并使用原路径重试登记。");
      setRegistration(result);
    },
  };
}

export type ProjectDerivationSession = ReturnType<typeof useProjectDerivation>;
