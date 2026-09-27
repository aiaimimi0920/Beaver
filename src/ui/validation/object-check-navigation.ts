import { createContext } from "react";
import type { ObjectAttemptCheckRequest } from "../../shared/object-attempt-checks";

// A report keeps its own identity; it must never be used as a validation run ID.
export const ObjectCheckNavigation = createContext<
  ((request: ObjectAttemptCheckRequest) => void) | undefined
>(undefined);
