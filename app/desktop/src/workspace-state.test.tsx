import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { flushWorkspaceStates, resetWorkspaceStores, updateWorkspaceState, useWorkspaceState } from "./workspace-state";
import DraftStatus from "./DraftStatus";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
let disk: Record<string, unknown>;
beforeEach(() => { disk = {}; vi.mocked(invoke).mockImplementation(async (command, args) => { const a = args as { key: string; value: unknown }; if (command === "load_workspace_state") return structuredClone(disk[a.key] ?? null); if (command === "save_workspace_state") { disk[a.key] = structuredClone(a.value); return; } }); });
function Editor() { const [draft,setDraft,state,retry]=useWorkspaceState("note-draft",{body:""}); return <><textarea aria-label="草稿" value={draft.body} onChange={e=>setDraft({body:e.target.value})}/><DraftStatus state={state} retry={retry}/></>; }
it("retains an uncommitted draft across navigation and a fresh store after restart", async () => {
  const view=render(<Editor/>); await screen.findByText("草稿自动保存在本机");
  fireEvent.change(screen.getByLabelText("草稿"),{target:{value:"PSRR 60 dB，待核查条件"}});
  view.unmount(); const second=render(<Editor/>); expect(screen.getByLabelText("草稿")).toHaveValue("PSRR 60 dB，待核查条件");
  await act(async()=>{await flushWorkspaceStates();}); expect(disk["note-draft"]).toEqual({body:"PSRR 60 dB，待核查条件"});
  second.unmount(); resetWorkspaceStores(); render(<Editor/>); await waitFor(()=>expect(screen.getByLabelText("草稿")).toHaveValue("PSRR 60 dB，待核查条件"));
});
it("serializes a newer edit behind a pending write and flush waits for both", async () => {
  let release:()=>void = ()=>{}; const first=new Promise<void>(resolve=>{release=resolve;}); let writes=0;
  vi.mocked(invoke).mockImplementation(async(command,args)=>{ const a=args as {key:string;value:unknown}; if(command==="load_workspace_state")return {body:"旧稿"}; if(command==="save_workspace_state") { writes++; if(writes===1)await first; disk[a.key]=structuredClone(a.value); } });
  const work=updateWorkspaceState("note-draft",{body:""},()=>({body:"第一稿"})); await waitFor(()=>expect(writes).toBe(1));
  const latest=updateWorkspaceState("note-draft",{body:""},()=>({body:"第二稿"})); release(); await Promise.all([work,latest]);
  expect(disk["note-draft"]).toEqual({body:"第二稿"}); expect(writes).toBe(2);
});
it("keeps text on write failure and allows an explicit retry", async () => {
  let fail=true; vi.mocked(invoke).mockImplementation(async(command,args)=>{if(command==="load_workspace_state")return null;if(fail)throw "磁盘写入失败"; const a=args as {key:string;value:unknown};disk[a.key]=a.value;});
  render(<Editor/>); await screen.findByText("草稿自动保存在本机"); fireEvent.change(screen.getByLabelText("草稿"),{target:{value:"不得丢失"}});
  expect(await screen.findByText("磁盘写入失败")).toBeInTheDocument(); expect(screen.getByLabelText("草稿")).toHaveValue("不得丢失");
  fail=false; fireEvent.click(screen.getByText("重试保存")); await screen.findByText("已保存到本机"); expect(disk["note-draft"]).toEqual({body:"不得丢失"});
});
it("reports malformed restored fields and does not overwrite the stored record automatically", async () => {
  disk["note-draft"]={body:42};render(<Editor/>);
  await screen.findByText("工作状态字段无效，请从备份恢复。");expect(screen.getByLabelText("草稿")).toHaveValue("");
  expect(disk["note-draft"]).toEqual({body:42});expect(vi.mocked(invoke).mock.calls.some(([command])=>command==="save_workspace_state")).toBe(false);
});
