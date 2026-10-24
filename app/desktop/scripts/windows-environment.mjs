// Keep the target OS claim separate from the availability of Windows APIs.
export function classifyWindows(system, scope = "win10") {
  if (!["windows", "win10"].includes(scope)) throw new Error("Use scope windows or win10.");
  const build = Number(system.BuildNumber);
  const product = Number(system.ProductType);
  if (!Number.isInteger(build) || build < 19044 || ![1, 2, 3].includes(product)) {
    throw new Error("Unsupported or unrecognized Windows environment.");
  }
  const family = product !== 1 ? "windows-server" : build < 22000 ? "windows-10" : "windows-11";
  if (scope === "win10" && family !== "windows-10") {
    throw new Error("Win10 acceptance requires a Windows 10 21H2/LTSC 2021 or 22H2 workstation.");
  }
  return { family, build, scope, win10Evidence: family === "windows-10" };
}
