export type EndpointStatus = "unchecked" | "valid" | "invalid" | "connected";

export type ModelEndpoint = {
  baseUrl: string;
  checkedAt: string;
  enabled: boolean;
  id: string;
  model: string;
  name: string;
  status: EndpointStatus;
  statusDetail: string;
};

export const modelEndpointsStorageKey = "ic-workbench-model-endpoints-v1";

export const defaultModelEndpoint: ModelEndpoint = {
  id: "local-openai-compatible",
  name: "本地 OpenAI 兼容服务",
  baseUrl: "http://127.0.0.1:11434/v1",
  model: "local-model",
  enabled: true,
  status: "unchecked",
  statusDetail: "尚未检查配置",
  checkedAt: "",
};

function isStatus(value: unknown): value is EndpointStatus {
  return (
    value === "unchecked" ||
    value === "valid" ||
    value === "invalid" ||
    value === "connected"
  );
}

function sanitizeEndpoint(value: unknown): ModelEndpoint | null {
  if (!value || typeof value !== "object") return null;
  const item = value as Partial<ModelEndpoint>;
  if (
    typeof item.id !== "string" ||
    typeof item.name !== "string" ||
    typeof item.baseUrl !== "string" ||
    typeof item.model !== "string"
  ) {
    return null;
  }

  return {
    id: item.id.slice(0, 80),
    name: item.name.trim().slice(0, 40) || "未命名端点",
    baseUrl: item.baseUrl.trim().slice(0, 500),
    model: item.model.trim().slice(0, 100),
    enabled: item.enabled !== false,
    status: isStatus(item.status) && item.status !== "connected" ? item.status : "unchecked",
    statusDetail:
      item.status === "connected"
        ? "需要重新执行连接检测"
        : typeof item.statusDetail === "string"
        ? item.statusDetail.slice(0, 160)
        : "尚未检查配置",
    checkedAt: typeof item.checkedAt === "string" ? item.checkedAt : "",
  };
}

export function loadModelEndpoints(): ModelEndpoint[] {
  try {
    const raw = window.localStorage.getItem(modelEndpointsStorageKey);
    if (!raw) return [defaultModelEndpoint];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [defaultModelEndpoint];
    const endpoints = parsed.map(sanitizeEndpoint).filter(Boolean) as ModelEndpoint[];
    return endpoints.length ? endpoints : [defaultModelEndpoint];
  } catch {
    return [defaultModelEndpoint];
  }
}

export function saveModelEndpoints(endpoints: ModelEndpoint[]): void {
  try {
    window.localStorage.setItem(modelEndpointsStorageKey, JSON.stringify(endpoints));
  } catch {
    // Keep the in-memory configuration usable when browser storage is unavailable.
  }
}

function isPrivateIpv4(hostname: string): boolean {
  const parts = hostname.split(".").map(Number);
  if (parts.length !== 4 || parts.some((part) => !Number.isInteger(part) || part < 0 || part > 255)) {
    return false;
  }
  return (
    parts[0] === 10 ||
    (parts[0] === 172 && parts[1] >= 16 && parts[1] <= 31) ||
    (parts[0] === 192 && parts[1] === 168) ||
    parts[0] === 127
  );
}

function isPrivateIpv6(hostname: string): boolean {
  const normalized = hostname.toLowerCase();
  if (normalized === "::1") return true;
  const firstBlock = normalized.split(":", 1)[0];
  const prefix = Number.parseInt(firstBlock, 16);
  return (
    Number.isInteger(prefix) &&
    ((prefix & 0xfe00) === 0xfc00 || (prefix & 0xffc0) === 0xfe80)
  );
}

export function validateIntranetEndpoint(baseUrl: string): string {
  let url: URL;
  try {
    url = new URL(baseUrl);
  } catch {
    return "请输入完整的 HTTP 或 HTTPS 地址。";
  }

  if (url.protocol !== "http:" && url.protocol !== "https:") {
    return "端点只允许使用 HTTP 或 HTTPS。";
  }
  if (url.username || url.password) {
    return "端点地址中不能包含用户名或密码。";
  }

  const hostname = url.hostname.toLowerCase().replace(/^\[|\]$/g, "");
  const allowed =
    hostname === "localhost" ||
    isPrivateIpv6(hostname) ||
    isPrivateIpv4(hostname) ||
    hostname.endsWith(".local") ||
    hostname.endsWith(".internal") ||
    (!hostname.includes(".") && /^[a-z0-9-]+$/.test(hostname));

  return allowed ? "" : "当前只允许本机、私有网段或内网主机名。";
}
