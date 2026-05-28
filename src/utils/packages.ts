import { invoke } from "@tauri-apps/api/core";

export interface Package {
  id: string;
  name: string;
  description: string;
  version: string;
  triggers: Array<{
    id: string;
    trigger_text: string;
    replacement: string;
  }>;
}

export async function installPackage(id: string): Promise<void> {
  await invoke("install_package", { id });
}

export async function uninstallPackage(id: string): Promise<void> {
  await invoke("uninstall_package", { id });
}

export async function loadPackageData(): Promise<{ packages: Package[]; installed: Set<string> }> {
  const [packages, installed] = await Promise.all([
    invoke<Package[]>("get_available_packages"),
    invoke<string[]>("get_installed_packages"),
  ]);
  return { packages, installed: new Set(installed) };
}
