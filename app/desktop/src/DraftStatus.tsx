export default function DraftStatus({ state, retry }: { state: { status: string; error: boolean }; retry: () => void }) {
  return <div className={`draft-status ${state.error ? "draft-error" : ""}`} role="status"><span>{state.status}</span>{state.error ? <button type="button" onClick={retry}>重试保存</button> : null}</div>;
}
