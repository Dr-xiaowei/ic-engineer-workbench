import { createServer } from "node:http";
import { pathToFileURL } from "node:url";

const LOOPBACK_HOST = "127.0.0.1";
const DEFAULT_PORT = 18080;
const MAX_REQUEST_BYTES = 1024 * 1024;
const MODEL_ID = "demo-analog-assistant";

function jsonResponse(response, status, payload, requestId) {
  const body = JSON.stringify(payload);
  response.writeHead(status, {
    "cache-control": "no-store",
    "content-length": Buffer.byteLength(body),
    "content-type": "application/json; charset=utf-8",
    "x-request-id": requestId,
  });
  response.end(body);
}

function requestText(content) {
  if (typeof content === "string") return content;
  if (!Array.isArray(content)) return "";
  return content
    .filter((part) => part && part.type === "text" && typeof part.text === "string")
    .map((part) => part.text)
    .join("\n");
}

function demoAnswer(payload) {
  const messages = Array.isArray(payload.messages) ? payload.messages : [];
  const latestUserMessage = [...messages]
    .reverse()
    .find((message) => message && message.role === "user");
  const prompt = requestText(latestUserMessage?.content).trim();
  const received = prompt ? `已收到问题摘要：“${prompt.slice(0, 80)}”` : "已收到合成测试请求";
  return [
    "这是本机确定性模拟响应，不是模型生成的工程结论。",
    "",
    `- ${received}`,
    "- 请求仅在 127.0.0.1 处理，没有访问内网或外网",
    "- 服务不保存提示词、附件或响应",
    "",
    "正式工程分析请切换到经过公司验收的内网模型，并复核来源、单位和条件。",
  ].join("\n");
}

function parsePayload(request) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    request.on("data", (chunk) => {
      size += chunk.length;
      if (size > MAX_REQUEST_BYTES) {
        reject(new Error("request-too-large"));
        request.destroy();
        return;
      }
      chunks.push(chunk);
    });
    request.on("end", () => {
      try {
        resolve(JSON.parse(Buffer.concat(chunks).toString("utf8")));
      } catch {
        reject(new Error("invalid-json"));
      }
    });
    request.on("error", reject);
  });
}

function validChatPayload(payload) {
  return (
    payload &&
    typeof payload === "object" &&
    typeof payload.model === "string" &&
    payload.model.length > 0 &&
    Array.isArray(payload.messages) &&
    payload.messages.length > 0 &&
    payload.messages.length <= 80 &&
    payload.store === false
  );
}

export function createDemoModelServer({ silent = false } = {}) {
  let sequence = 0;
  const server = createServer(async (request, response) => {
    sequence += 1;
    const requestId = `demo-request-${String(sequence).padStart(4, "0")}`;
    const url = new URL(request.url ?? "/", `http://${LOOPBACK_HOST}`);
    const finish = (status) => {
      if (!silent) console.log(`${request.method ?? "UNKNOWN"} ${url.pathname} ${status}`);
    };

    response.setHeader("connection", "close");
    if (request.method === "GET" && url.pathname === "/v1/models") {
      jsonResponse(
        response,
        200,
        { object: "list", data: [{ id: MODEL_ID, object: "model", owned_by: "local-demo" }] },
        requestId,
      );
      finish(200);
      return;
    }

    if (request.method !== "POST" || url.pathname !== "/v1/chat/completions") {
      jsonResponse(response, 404, { error: { message: "仅支持 Demo 模型接口。" } }, requestId);
      finish(404);
      return;
    }

    let payload;
    try {
      payload = await parsePayload(request);
    } catch (error) {
      const tooLarge = error instanceof Error && error.message === "request-too-large";
      if (!response.destroyed) {
        jsonResponse(
          response,
          tooLarge ? 413 : 400,
          { error: { message: tooLarge ? "请求超过 1 MiB。" : "请求 JSON 无效。" } },
          requestId,
        );
      }
      finish(tooLarge ? 413 : 400);
      return;
    }

    if (!validChatPayload(payload)) {
      jsonResponse(
        response,
        400,
        { error: { message: "请求必须包含模型、消息并显式设置 store=false。" } },
        requestId,
      );
      finish(400);
      return;
    }

    const answer = demoAnswer(payload);
    if (payload.stream === true) {
      response.writeHead(200, {
        "cache-control": "no-cache, no-store",
        connection: "close",
        "content-type": "text/event-stream; charset=utf-8",
        "x-accel-buffering": "no",
        "x-request-id": requestId,
      });
      for (const delta of [answer.slice(0, 36), answer.slice(36)]) {
        response.write(
          `data: ${JSON.stringify({ choices: [{ delta: { content: delta } }] })}\n\n`,
        );
      }
      response.end("data: [DONE]\n\n");
      finish(200);
      return;
    }

    jsonResponse(
      response,
      200,
      { choices: [{ message: { role: "assistant", content: answer } }] },
      requestId,
    );
    finish(200);
  });
  server.maxHeadersCount = 64;
  server.headersTimeout = 5_000;
  server.requestTimeout = 10_000;
  return server;
}

function parsePort(argumentsList) {
  const index = argumentsList.indexOf("--port");
  if (index === -1) return DEFAULT_PORT;
  const port = Number(argumentsList[index + 1]);
  if (!Number.isInteger(port) || port < 1024 || port > 65535) {
    throw new Error("端口必须是 1024 到 65535 之间的整数。");
  }
  return port;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  let port;
  try {
    port = parsePort(process.argv.slice(2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(2);
  }
  const server = createDemoModelServer();
  server.listen(port, LOOPBACK_HOST, () => {
    console.log(`本机模型 Demo 已启动：http://${LOOPBACK_HOST}:${port}/v1`);
    console.log(`模型标识：${MODEL_ID}；按 Ctrl+C 停止。`);
  });
  const stop = () => server.close(() => process.exit(0));
  process.once("SIGINT", stop);
  process.once("SIGTERM", stop);
}
