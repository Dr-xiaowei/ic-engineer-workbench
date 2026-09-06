import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { createDemoModelServer } from "./mock-model-server.mjs";

let baseUrl;
let server;

before(async () => {
  server = createDemoModelServer({ silent: true });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  assert.equal(typeof address, "object");
  assert.equal(address.address, "127.0.0.1");
  baseUrl = `http://127.0.0.1:${address.port}/v1`;
});

after(async () => {
  await new Promise((resolve) => server.close(resolve));
});

test("提供 OpenAI 兼容模型列表且禁止缓存", async () => {
  const response = await fetch(`${baseUrl}/models`);
  assert.equal(response.status, 200);
  assert.equal(response.headers.get("cache-control"), "no-store");
  const payload = await response.json();
  assert.equal(payload.data[0].id, "demo-analog-assistant");
});

test("非流式响应明确标注为本机模拟内容", async () => {
  const response = await fetch(`${baseUrl}/chat/completions`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      model: "demo-analog-assistant",
      messages: [{ role: "user", content: "验证偏置电流" }],
      stream: false,
      store: false,
    }),
  });
  assert.equal(response.status, 200);
  const payload = await response.json();
  assert.match(payload.choices[0].message.content, /确定性模拟响应/);
  assert.match(payload.choices[0].message.content, /偏置电流/);
});

test("流式响应包含增量事件和结束标记", async () => {
  const response = await fetch(`${baseUrl}/chat/completions`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      model: "demo-analog-assistant",
      messages: [{ role: "user", content: "流式协议检查" }],
      stream: true,
      store: false,
    }),
  });
  assert.equal(response.headers.get("content-type"), "text/event-stream; charset=utf-8");
  const body = await response.text();
  assert.match(body, /"delta":\{"content":/);
  assert.match(body, /data: \[DONE\]/);
});

test("拒绝可能远端保存请求的负向样例", async () => {
  const response = await fetch(`${baseUrl}/chat/completions`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      model: "demo-analog-assistant",
      messages: [{ role: "user", content: "不应保存" }],
      stream: false,
      store: true,
    }),
  });
  assert.equal(response.status, 400);
});
