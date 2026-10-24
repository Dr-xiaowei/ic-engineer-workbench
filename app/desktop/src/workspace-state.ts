import { useSyncExternalStore, type SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { validateTemplate } from "./work-templates";

type Snapshot<T> = { value: T; ready: boolean; status: string; error: boolean };
type Store<T> = { snapshot: Snapshot<T>; listeners: Set<() => void>; loading: Promise<void>; pending: boolean; running: boolean; inflight?: Promise<void>; revision: number; timer?: ReturnType<typeof setTimeout> };
const stores = new Map<string, Store<unknown>>();
const message = (error: unknown) => typeof error === "string" ? error : error instanceof Error ? error.message : "本地草稿保存不可用，请保持页面并重试。";
function emit<T>(store: Store<T>, patch: Partial<Snapshot<T>>) { store.snapshot = { ...store.snapshot, ...patch }; store.listeners.forEach(fn => fn()); }
function restoreShape<T>(key: string, value: unknown, initial: T): T {
  if (value == null) return initial;
  if (typeof initial === "object" && initial !== null && !Array.isArray(initial)) {
    if (!value || typeof value !== "object" || Array.isArray(value)) throw "工作状态格式损坏，请保留备份并恢复。";
    const record = value as Record<string, unknown>;
    for (const [field, fallback] of Object.entries(initial)) {
      const item = record[field]; if (item === undefined || fallback === null) continue;
      if (Array.isArray(fallback) ? !Array.isArray(item) : typeof item !== typeof fallback || (typeof item === "number" && !Number.isFinite(item))) throw "工作状态字段无效，请从备份恢复。";
    }
    const citations = (record.citations ?? record.highlights ?? []) as unknown[];
    if (!Array.isArray(citations) || citations.some(c => { const v = c as Record<string,unknown>; return !v || ["sourceId","sourceName","version","quote"].some(f => typeof v[f] !== "string") || !Number.isInteger(v.page) || Number(v.page) < 1; })) throw "原文引用草稿损坏，请从备份恢复。";
    if (key === "templates") { for (const t of (record.custom ?? []) as unknown[]) validateTemplate(t); }
    return { ...initial, ...record };
  }
  if (typeof value !== typeof initial) throw "工作草稿格式无效。";
  return value as T;
}
function getStore<T>(key: string, initial: T): Store<T> {
  const existing = stores.get(key) as Store<T> | undefined;
  if (existing) return existing;
  const store: Store<T> = { snapshot: { value: initial, ready: false, status: "正在恢复草稿…", error: false }, listeners: new Set(), loading: Promise.resolve(), pending: false, running: false, revision: 0 };
  stores.set(key, store as Store<unknown>);
  store.loading = invoke<T | null>("load_workspace_state", { key }).then(value => {
    emit(store, { value: store.revision ? store.snapshot.value : restoreShape(key, value, initial), ready: true, status: value == null ? "草稿自动保存在本机" : "已恢复本机工作状态", error: false });
  }).catch(error => emit(store, { ready: true, status: message(error), error: true }));
  return store;
}
async function flush<T>(key: string, store: Store<T>): Promise<void> {
  await store.loading;
  if (store.inflight) { await store.inflight; if (store.pending) return flush(key, store); return; }
  const run = async () => {
    while (store.pending) {
      store.pending = false;
      const value = store.snapshot.value, revision = store.revision;
      try {
        await invoke("save_workspace_state", { key, value });
        if (revision === store.revision) emit(store, { status: "已保存到本机", error: false });
      } catch (error) {
        store.pending = false;
        emit(store, { status: message(error), error: true });
        break;
      }
    }
  };
  store.inflight = run();
  await store.inflight;
  store.inflight = undefined;
}

export function setWorkspaceState<T>(key: string, initial: T, update: SetStateAction<T>) {
  const store = getStore(key, initial);
  const value = typeof update === "function" ? (update as (value: T) => T)(store.snapshot.value) : update;
  store.revision += 1;
  store.pending = true;
  emit(store, { value, status: "正在保存…", error: false });
  clearTimeout(store.timer);
  store.timer = setTimeout(() => void flush(key, store), 150);
}
export async function updateWorkspaceState<T>(key: string, initial: T, update: (value: T) => T) {
  const store = getStore(key, initial);
  await store.loading;
  setWorkspaceState(key, initial, update);
  await flush(key, store);
  if (store.snapshot.error) throw new Error(store.snapshot.status);
}
export function useWorkspaceState<T>(key: string, initial: T) {
  const store = getStore(key, initial);
  const snapshot = useSyncExternalStore(fn => { store.listeners.add(fn); return () => store.listeners.delete(fn); }, () => store.snapshot);
  const set = (update: SetStateAction<T>) => setWorkspaceState(key, initial, update);
  const retry = () => { store.pending = true; void flush(key, store); };
  return [snapshot.value, set, snapshot, retry] as const;
}
export function resetWorkspaceStores() { stores.forEach(s => clearTimeout(s.timer)); stores.clear(); }
export function useDraftField<T, K extends keyof T>(key: string, initial: T, field: K) {
  const [value, set, state, retry] = useWorkspaceState(key, initial);
  const update = (next: SetStateAction<T[K]>) => set(current => ({ ...current, [field]: typeof next === "function" ? (next as (v: T[K]) => T[K])(current[field]) : next }));
  return [value[field], update, state, retry] as const;
}

export async function flushWorkspaceStates() {
  for (const [key, store] of stores) {
    clearTimeout(store.timer);
    await flush(key, store);
    if (store.snapshot.error && store.revision) throw new Error(store.snapshot.status);
  }
}
