import { useState, useEffect } from "react";
import { Zap, Hash, BookOpen, Download, Upload, Settings, Package, ArrowUpFromLine, Terminal, Loader, ChevronLeft, ChevronRight } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";
import { check, Update } from "@tauri-apps/plugin-updater";
import { useStore } from "../store";
import { t } from "../i18n";

export function Sidebar() {
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);
  const triggers = useStore((s) => s.triggers);
  const globalVars = useStore((s) => s.globalVars);
  const lang = useStore((s) => s.settings.language);
  const loadData = useStore((s) => s.loadData);
  const collapsed = useStore((s) => s.sidebarCollapsed);
  const toggleSidebar = useStore((s) => s.toggleSidebar);
  const triggerCount = triggers.length;
  const globalVarCount = globalVars.length;
  const [update, setUpdate] = useState<Update | null>(null);
  const [updateState, setUpdateState] = useState<"idle" | "downloading" | "error">("idle");

  useEffect(() => {
    check().then((u) => setUpdate(u)).catch(() => setUpdateState("error"));
  }, []);

  async function handleUpdate() {
    if (!update) return;
    setUpdateState("downloading");
    try {
      await update.downloadAndInstall();
    } catch {
      setUpdateState("error");
    }
  }

  async function handleExport() {
    try {
      const filePath = await save({
        title: "Export Data",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!filePath) return;
      await invoke("export_data_to_file", { path: filePath });
    } catch (e) {
      console.error("Export failed:", e);
    }
  }

  async function handleImport() {
    try {
      const filePath = await open({
        title: "Import Data",
        filters: [{ name: "JSON", extensions: ["json"] }],
        multiple: false,
      });
      if (!filePath) return;
      const json = await readTextFile(filePath as string);
      const result = await invoke<string>("import_data", { json });
      alert(result);
      loadData();
    } catch (e) {
      console.error("Import failed:", e);
    }
  }

  return (
    <aside className={`sidebar ${collapsed ? "collapsed" : ""}`}>
      <div className="sidebar-header">
        <div className="logo">
          <LogoIcon size={collapsed ? 24 : 28} />
          {!collapsed && <span className="logo-text">trigr</span>}
        </div>
        {!collapsed && (
          <>
            <div className="sidebar-header-spacer" />
            <button
              className="sidebar-collapse-btn"
              onClick={toggleSidebar}
              title="Collapse sidebar"
            >
              <ChevronLeft size={14} />
            </button>
          </>
        )}
      </div>
      <nav className="sidebar-nav">
        <button
          className={`nav-item ${view === "triggers" ? "active" : ""}`}
          onClick={() => setView("triggers")}
          title={collapsed ? t("sidebar.triggers", lang) : undefined}
        >
          <Zap size={16} />
          {!collapsed && <span className="nav-label">{t("sidebar.triggers", lang)}</span>}
          {!collapsed && <span className="nav-badge">{triggerCount}</span>}
        </button>
        <button
          className={`nav-item ${view === "globalvars" ? "active" : ""}`}
          onClick={() => setView("globalvars")}
          title={collapsed ? t("sidebar.variables", lang) : undefined}
        >
          <Hash size={16} />
          {!collapsed && <span className="nav-label">{t("sidebar.variables", lang)}</span>}
          {!collapsed && <span className="nav-badge">{globalVarCount}</span>}
        </button>
        <button
          className={`nav-item ${view === "scriptlang" ? "active" : ""}`}
          onClick={() => setView("scriptlang")}
          title={collapsed ? t("sidebar.script", lang) : undefined}
        >
          <BookOpen size={16} />
          {!collapsed && <span className="nav-label">{t("sidebar.script", lang)}</span>}
        </button>
        <button
          className={`nav-item ${view === "scriptrunner" ? "active" : ""}`}
          onClick={() => setView("scriptrunner")}
          title={collapsed ? t("sidebar.scriptrunner", lang) : undefined}
        >
          <Terminal size={16} />
          {!collapsed && <span className="nav-label">{t("sidebar.scriptrunner", lang)}</span>}
        </button>
        <button
          className={`nav-item ${view === "packages" ? "active" : ""}`}
          onClick={() => setView("packages")}
          title={collapsed ? t("sidebar.packages", lang) : undefined}
        >
          <Package size={16} />
          {!collapsed && <span className="nav-label">{t("sidebar.packages", lang)}</span>}
        </button>
        <button
          className={`nav-item ${view === "settings" ? "active" : ""}`}
          onClick={() => setView("settings")}
          title={collapsed ? t("sidebar.settings", lang) : undefined}
        >
          <Settings size={16} />
          {!collapsed && <span className="nav-label">{t("sidebar.settings", lang)}</span>}
        </button>
      </nav>
      <div className="sidebar-footer">
        {update && updateState !== "error" && (
          <div
            className="update-banner"
            onClick={handleUpdate}
            title={collapsed ? `Update to ${update.version}` : `Update to ${update.version}`}
          >
            {updateState === "downloading" ? (
              <Loader size={14} className="update-spinner" />
            ) : (
              <ArrowUpFromLine size={14} />
            )}
            {!collapsed && <span>{updateState === "downloading" ? "Downloading..." : "Update Available"}</span>}
          </div>
        )}
        {updateState === "error" && (
          <div className="update-banner update-banner-error" onClick={handleUpdate} title="Retry update">
            <ArrowUpFromLine size={14} />
            {!collapsed && <span>Update failed - try again</span>}
          </div>
        )}
        <button className="nav-item data-btn" onClick={handleExport} title={collapsed ? t("sidebar.export", lang) : undefined}>
          <Download size={14} />
          {!collapsed && <span>{t("sidebar.export", lang)}</span>}
        </button>
          <button className="nav-item data-btn" onClick={handleImport} title={collapsed ? t("sidebar.import", lang) : undefined}>
            <Upload size={14} />
            {!collapsed && <span>{t("sidebar.import", lang)}</span>}
          </button>
          <button
            className="nav-item sidebar-expand-btn"
            onClick={toggleSidebar}
            title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          >
            {collapsed ? <ChevronRight size={14} /> : <ChevronLeft size={14} />}
          </button>
        </div>
      </aside>
  );
}

function LogoIcon({ size }: { size: number }) {
  return (
    <svg
      className="logo-icon"
      width={size}
      height={size}
      viewBox="0 0 512 512"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <rect x="48" y="48" width="416" height="416" rx="96" fill="#1a1a2e" />
      <path d="M192,160 L152,160 L120,216 L120,296 L152,352 L192,352" stroke="#8b5cf6" strokeWidth="32" fill="none" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M320,160 L360,160 L392,216 L392,296 L360,352 L320,352" stroke="#8b5cf6" strokeWidth="32" fill="none" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M272,128 L208,256 L248,256 L232,384 L336,240 L276,240 Z" fill="#a78bfa" stroke="#ffffff" strokeWidth="8" strokeLinejoin="round" />
    </svg>
  );
}
