import { useSyncExternalStore } from "react";

let verifiedEndpointIds: readonly string[] = [];
const listeners = new Set<() => void>();

function emitChange() {
  for (const listener of listeners) listener();
}

export function markEndpointVerified(endpointId: string) {
  if (verifiedEndpointIds.includes(endpointId)) return;
  verifiedEndpointIds = [...verifiedEndpointIds, endpointId];
  emitChange();
}

export function markEndpointUnverified(endpointId: string) {
  if (!verifiedEndpointIds.includes(endpointId)) return;
  verifiedEndpointIds = verifiedEndpointIds.filter((id) => id !== endpointId);
  emitChange();
}

export function clearVerifiedEndpoints() {
  if (!verifiedEndpointIds.length) return;
  verifiedEndpointIds = [];
  emitChange();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function getSnapshot() {
  return verifiedEndpointIds;
}

export function useVerifiedEndpointIds() {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
