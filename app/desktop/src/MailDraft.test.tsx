import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import MailWorkspace, { mailFolderLabel } from "./MailWorkspace";
import { flushWorkspaceStates, resetWorkspaceStores } from "./workspace-state";
vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn()}));
vi.mock("@tauri-apps/plugin-dialog",()=>({open:vi.fn(),save:vi.fn()}));
let disk:Record<string,unknown>;
beforeEach(()=>{disk={};vi.mocked(invoke).mockImplementation(async(command,args)=>{
  const a=args as {key:string;value:unknown};
  if(command==="load_workspace_state")return disk[a.key]??null;
  if(command==="save_workspace_state"){disk[a.key]=structuredClone(a.value);return;}
  if(command==="list_mail_accounts")return [{id:"mail-test",address:"engineer@example.invalid",displayName:"合成邮箱",username:"test",imapHost:"mail.internal",imapPort:993,smtpHost:"mail.internal",smtpPort:465,updatedAt:0}];
  if(command==="has_mail_secrets")return true;
  if(command==="list_local_mail")return [];
});});
it("restores an unsent mail and its sender after closing the composer and restarting, without sending",async()=>{
  const first=render(<MailWorkspace/>);fireEvent.click(await screen.findByRole("button",{name:"写邮件"}));
  fireEvent.change(screen.getByLabelText("收件人"),{target:{value:"review@example.invalid"}});
  fireEvent.change(screen.getByLabelText("主题"),{target:{value:"封装边界核查"}});
  fireEvent.change(screen.getByLabelText("正文"),{target:{value:"仅合成测试，保留待确认的单位与条件。"}});
  fireEvent.click(screen.getByLabelText("关闭写邮件"));
  await act(flushWorkspaceStates);first.unmount();resetWorkspaceStores();
  render(<MailWorkspace/>);fireEvent.click(await screen.findByRole("button",{name:"继续邮件草稿"}));
  await waitFor(()=>expect(screen.getByLabelText("主题")).toHaveValue("封装边界核查"));
  expect(screen.getByLabelText("草稿发件账户")).toHaveValue("mail-test");
  expect(vi.mocked(invoke).mock.calls.some(([c])=>c==="send_mail")).toBe(false);
  fireEvent.click(screen.getByRole("button",{name:"检查并准备发送"}));
  expect(await screen.findByRole("heading",{name:"确认发送这封邮件？"})).toBeInTheDocument();
  expect(vi.mocked(invoke).mock.calls.some(([c])=>c==="send_mail")).toBe(false);
  expect(disk["mail-draft"]).toEqual(expect.objectContaining({accountId:"mail-test",composer:expect.objectContaining({subject:"封装边界核查"})}));
});

it("displays IMAP international folder labels without changing server identifiers",()=>{
  expect(mailFolderLabel("&U,BTFw-")).toBe("台北");
  expect(mailFolderLabel("&ZeVnLIqe-")).toBe("日本語");
  expect(mailFolderLabel("R&-D/已发送")).toBe("R&D/已发送");
  expect(mailFolderLabel("INBOX")).toBe("INBOX");
  expect(mailFolderLabel("&invalid!-")).toBe("&invalid!-");
});

it.each([true, false])("reports simulated=%s delivery accurately after explicit confirmation", async(simulated)=>{
  const fallback=vi.mocked(invoke).getMockImplementation()!;
  vi.mocked(invoke).mockImplementation(async(command,args)=>command==="send_mail"?{simulated}:fallback(command,args));
  render(<MailWorkspace/>);fireEvent.click(await screen.findByRole("button",{name:"写邮件"}));
  fireEvent.change(screen.getByLabelText("收件人"),{target:{value:"test@example.invalid"}});
  fireEvent.change(screen.getByLabelText("主题"),{target:{value:"合成发送"}});
  fireEvent.change(screen.getByLabelText("正文"),{target:{value:"仅本机验收"}});
  fireEvent.click(screen.getByRole("button",{name:"检查并准备发送"}));
  expect(vi.mocked(invoke).mock.calls.some(([c])=>c==="send_mail")).toBe(false);
  fireEvent.click(await screen.findByRole("button",{name:"确认并发送"}));
  expect(await screen.findByText(simulated?"Demo 发送模拟已完成；没有连接 SMTP，也没有投递真实邮件。":"SMTP 已确认接收邮件。可同步已发送文件夹核对服务端副本。")).toBeInTheDocument();
  expect(vi.mocked(invoke).mock.calls.filter(([c])=>c==="send_mail")).toHaveLength(1);
  expect(screen.queryByRole("heading",{name:"写邮件"})).not.toBeInTheDocument();
});
