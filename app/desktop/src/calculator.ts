export type AngleMode = "deg" | "rad";

type Token = { kind: "number" | "name" | "symbol" | "end"; text: string; value?: number };

function tokenize(source: string): Token[] {
  if (!source.trim() || source.length > 500) throw new Error("表达式不能为空且不能超过 500 字符。");
  const tokens: Token[] = [];
  let index = 0;
  while (index < source.length) {
    const rest = source.slice(index);
    const space = rest.match(/^\s+/);
    if (space) { index += space[0].length; continue; }
    const number = rest.match(/^(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?/i);
    if (number) {
      const value = Number(number[0]);
      if (!Number.isFinite(value)) throw new Error("数字超出可计算范围。");
      tokens.push({ kind: "number", text: number[0], value }); index += number[0].length; continue;
    }
    const name = rest.match(/^[a-zA-Z_][a-zA-Z0-9_]*/);
    if (name) { tokens.push({ kind: "name", text: name[0].toLowerCase() }); index += name[0].length; continue; }
    const symbol = rest[0];
    if ("+-*/^%(),".includes(symbol)) { tokens.push({ kind: "symbol", text: symbol }); index += 1; continue; }
    throw new Error(`不支持的字符：${symbol}`);
  }
  tokens.push({ kind: "end", text: "" });
  return tokens;
}

export function calculateExpression(source: string, angleMode: AngleMode): number {
  const tokens = tokenize(source);
  let position = 0;
  const current = () => tokens[position];
  const take = (text?: string) => {
    const token = current();
    if (text && token.text !== text) throw new Error(`此处应为“${text}”。`);
    position += 1; return token;
  };
  const angleIn = (value: number) => angleMode === "deg" ? value * Math.PI / 180 : value;
  const angleOut = (value: number) => angleMode === "deg" ? value * 180 / Math.PI : value;
  const functions: Record<string, (...args: number[]) => number> = {
    sin: (x) => Math.sin(angleIn(x)), cos: (x) => Math.cos(angleIn(x)), tan: (x) => Math.tan(angleIn(x)),
    asin: (x) => angleOut(Math.asin(x)), acos: (x) => angleOut(Math.acos(x)), atan: (x) => angleOut(Math.atan(x)),
    sqrt: Math.sqrt, abs: Math.abs, ln: Math.log, log: Math.log10, exp: Math.exp,
    floor: Math.floor, ceil: Math.ceil, round: Math.round, pow: Math.pow, min: Math.min, max: Math.max,
  };
  function primary(): number {
    const token = current();
    if (token.kind === "number") { take(); return token.value!; }
    if (token.kind === "name") {
      const name = take().text;
      if (current().text !== "(") {
        if (name === "pi") return Math.PI;
        if (name === "e") return Math.E;
        throw new Error(`未知常量：${name}`);
      }
      take("(");
      const args: number[] = [];
      if (current().text !== ")") {
        args.push(expression());
        while (current().text === ",") { take(","); args.push(expression()); }
      }
      take(")");
      const fn = functions[name];
      if (!fn) throw new Error(`未知函数：${name}`);
      if ((["pow", "min", "max"].includes(name) && args.length < 2) || (!["pow", "min", "max"].includes(name) && args.length !== 1)) {
        throw new Error(`函数 ${name} 的参数数量不正确。`);
      }
      return fn(...args);
    }
    if (token.text === "(") { take("("); const value = expression(); take(")"); return value; }
    throw new Error("表达式不完整。");
  }
  function power(): number { const left = primary(); if (current().text === "^") { take(); return Math.pow(left, unary()); } return left; }
  function unary(): number { if (current().text === "+") { take(); return unary(); } if (current().text === "-") { take(); return -unary(); } return power(); }
  function product(): number { let value = unary(); while (["*", "/", "%"].includes(current().text)) { const op = take().text; const right = unary(); if ((op === "/" || op === "%") && right === 0) throw new Error("不能除以零。"); value = op === "*" ? value * right : op === "/" ? value / right : value % right; } return value; }
  function expression(): number { let value = product(); while (["+", "-"].includes(current().text)) { const op = take().text; const right = product(); value = op === "+" ? value + right : value - right; } return value; }
  const result = expression();
  if (current().kind !== "end") throw new Error(`无法解析“${current().text}”之后的内容。`);
  if (!Number.isFinite(result)) throw new Error("结果不是有限数值，请检查定义域或数值范围。");
  return Object.is(result, -0) ? 0 : result;
}
