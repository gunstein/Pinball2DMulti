/**
 * Deep space module - exports backend interfaces and implementations.
 */

export type {
  DeepSpaceBackend,
  CaptureEvent,
  CaptureCallback,
  ConnectionState,
} from "./DeepSpaceBackend";

export { LocalDeepSpaceBackend } from "./LocalDeepSpaceBackend";
export { ServerDeepSpaceBackend } from "./ServerDeepSpaceBackend";
