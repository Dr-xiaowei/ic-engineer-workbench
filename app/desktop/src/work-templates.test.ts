import { expect, it } from "vitest";
import { builtInTemplates, fillTemplate, validateTemplate } from "./work-templates";
it("provides four usable IC workflows with explicit missing evidence",()=>{
  expect(builtInTemplates.map(t=>t.name)).toEqual(["设计评审","Datasheet 核查","故障排查","项目周报"]);
  for(const template of builtInTemplates){expect(validateTemplate(template)).toEqual(template);expect(fillTemplate(template,{})).toContain("待补充");expect(fillTemplate(template,{})).not.toContain("{{");}
});
it("substitutes user text literally and rejects undefined fields or executable permissions",()=>{
  const template=builtInTemplates[0]; expect(fillTemplate(template,{project:"$& {{goal}} <script>"})).toContain("$& {{goal}} <script>");
  expect(()=>validateTemplate({...template,body:"{{unknown}}"})).toThrow("没有对应字段");
  expect(()=>validateTemplate({...template,permissions:["shell"]})).toThrow("不接受脚本");
  expect(()=>validateTemplate({...template,commands:["sh"]})).toThrow();
  expect(()=>validateTemplate({...template,fields:[template.fields[0],template.fields[0]]})).toThrow("唯一");
});
