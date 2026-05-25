import { useState, useEffect, useCallback, useRef } from "react";
import { Keyboard, Palette, Check, Minus, Plus, ChevronDown, RefreshCw, Download, Loader } from "lucide-react";
import { BlossomColorPicker } from "@dayflow/blossom-color-picker-react";
import "../blossom-color-picker.css";
import { check, Update } from "@tauri-apps/plugin-updater";
import { getVersion } from "@tauri-apps/api/app";
import { applyThemeColors, hexToBlossom } from "../utils/color";
import { t, languages } from "../i18n";
import { useStore } from "../store";

export function SettingsView() {
  const storeSettings = useStore((s) => s.settings);
  const updateSettings = useStore((s) => s.updateSettings);
  const lang = storeSettings.language;
  const [enderChar, setEnderChar] = useState(storeSettings.ender_char);
  const [themeColor, setThemeColor] = useState(storeSettings.theme_color);
  const [fontSize, setFontSize] = useState(storeSettings.font_size);
  const [fontSizeInput, setFontSizeInput] = useState(String(storeSettings.font_size));
  const [saved, setSaved] = useState(false);
  const [showColorPicker, setShowColorPicker] = useState(false);
  const colorControlRef = useRef<HTMLDivElement>(null);
  const saveTimer = useRef<number | undefined>(undefined);

  const [updateStatus, setUpdateStatus] = useState<"idle" | "checking" | "available" | "uptodate" | "downloading" | "error">("idle");
  const [availableUpdate, setAvailableUpdate] = useState<Update | null>(null);
  const [currentVersion, setCurrentVersion] = useState("");

  useEffect(() => {
    getVersion().then(setCurrentVersion).catch(() => setCurrentVersion("0.1.4"));
  }, []);

  async function checkForUpdates() {
    setUpdateStatus("checking");
    try {
      const u = await check();
      if (u) {
        setAvailableUpdate(u);
        setUpdateStatus("available");
      } else {
        setUpdateStatus("uptodate");
      }
    } catch {
      setUpdateStatus("error");
    }
  }

  async function handleInstall() {
    if (!availableUpdate) return;
    setUpdateStatus("downloading");
    try {
      await availableUpdate.downloadAndInstall();
    } catch {
      setUpdateStatus("error");
    }
  }

  const saveSettings = useCallback(async (ender: string, color: string, size: number, language: string) => {
    await updateSettings({ ender_char: ender, theme_color: color, font_size: size, language });
    applyThemeColors(color);
    document.documentElement.style.fontSize = `${size}px`;
    setSaved(true);
    if (saveTimer.current) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => setSaved(false), 1500);
  }, [updateSettings]);

  useEffect(() => {
    setEnderChar(storeSettings.ender_char);
    setThemeColor(storeSettings.theme_color);
    setFontSize(storeSettings.font_size);
    setFontSizeInput(String(storeSettings.font_size));
    applyThemeColors(storeSettings.theme_color);
    document.documentElement.style.fontSize = `${storeSettings.font_size}px`;
  }, [storeSettings]);

  useEffect(() => {
    if (!showColorPicker) return;
    function handleClickOutside(e: MouseEvent) {
      if (
        colorControlRef.current &&
        !colorControlRef.current.contains(e.target as Node)
      ) {
        setShowColorPicker(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [showColorPicker]);

  useEffect(() => {
    return () => { if (saveTimer.current) clearTimeout(saveTimer.current); };
  }, []);

  const blossomValue = hexToBlossom(themeColor);

  const handleColorChange = useCallback((color: { hex: string }) => {
    setThemeColor(color.hex);
    saveSettings(enderChar, color.hex, fontSize, lang);
  }, [enderChar, fontSize, lang, saveSettings]);

  const handleEnderChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value.slice(-1);
    setEnderChar(val);
    saveSettings(val, themeColor, fontSize, lang);
  }, [themeColor, fontSize, lang, saveSettings]);

  const handleFontSizeSlider = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const val = Number(e.target.value);
    setFontSize(val);
    setFontSizeInput(String(val));
    saveSettings(enderChar, themeColor, val, lang);
  }, [enderChar, themeColor, lang, saveSettings]);

  const handleFontSizeInput = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setFontSizeInput(e.target.value);
    const val = parseInt(e.target.value, 10);
    if (!isNaN(val) && val >= 12 && val <= 20) {
      setFontSize(val);
      saveSettings(enderChar, themeColor, val, lang);
    }
  }, [enderChar, themeColor, lang, saveSettings]);

  const handleFontSizeBlur = useCallback(() => {
    const val = parseInt(fontSizeInput, 10);
    if (isNaN(val) || val < 12) {
      setFontSize(12);
      setFontSizeInput("12");
      saveSettings(enderChar, themeColor, 12, lang);
    } else if (val > 20) {
      setFontSize(20);
      setFontSizeInput("20");
      saveSettings(enderChar, themeColor, 20, lang);
    } else {
      setFontSize(val);
      setFontSizeInput(String(val));
    }
  }, [fontSizeInput, enderChar, themeColor, lang, saveSettings]);

  const handleFontSizeStep = useCallback((dir: number) => {
    const val = Math.min(20, Math.max(12, fontSize + dir));
    setFontSize(val);
    setFontSizeInput(String(val));
    saveSettings(enderChar, themeColor, val, lang);
  }, [fontSize, enderChar, themeColor, lang, saveSettings]);

  const handleLangChange = useCallback((val: string) => {
    updateSettings({ language: val });
  }, [updateSettings]);

  return (
    <div className="view-container">
      <div className="view-header">
        <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
          <div style={{ flex: 1 }}>
            <h1>{t("settings.title", lang)}</h1>
            <p>{t("settings.subtitle", lang)}</p>
          </div>
          {saved && (
            <span style={{ display: "flex", alignItems: "center", gap: "0.375rem", fontSize: "0.75rem", color: "var(--success)", fontWeight: 600 }}>
              <Check size={14} /> Saved
            </span>
          )}
        </div>
      </div>

      <div className="settings-card">
        <div className="settings-card-header settings-card-header-first">
          <Palette size={16} />
          <span>{t("settings.appearance", lang)}</span>
        </div>
        <div className="settings-row">
          <div className="settings-row-label">
            <div className="row-title">{t("settings.theme_color", lang)}</div>
            <div className="row-desc">{t("settings.theme_color_desc", lang)}</div>
          </div>
          <div className="theme-color-control" ref={colorControlRef}>
            <button
              className="color-swatch-btn"
              style={{ backgroundColor: themeColor }}
              onClick={() => setShowColorPicker(!showColorPicker)}
            />
            {showColorPicker && (
              <div className="color-picker-dropdown">
                <BlossomColorPicker
                  value={blossomValue}
                  onChange={handleColorChange}
                  initialExpanded={true}
                />
              </div>
            )}
          </div>
        </div>
        <div className="settings-row settings-row-last">
          <div className="settings-row-label">
            <div className="row-title">{t("settings.font_size", lang)}</div>
            <div className="row-desc">{t("settings.font_size_desc", lang)}</div>
          </div>
          <div className="font-size-control">
            <button className="font-size-step" onClick={() => handleFontSizeStep(-1)} title="Decrease">
              <Minus size={12} />
            </button>
            <input
              type="range"
              min={12}
              max={20}
              step={1}
              value={fontSize}
              onChange={handleFontSizeSlider}
              className="font-size-slider"
            />
            <button className="font-size-step" onClick={() => handleFontSizeStep(1)} title="Increase">
              <Plus size={12} />
            </button>
            <input
              type="text"
              value={fontSizeInput}
              onChange={handleFontSizeInput}
              onBlur={handleFontSizeBlur}
              className="font-size-input"
            />
            <span className="row-desc">px</span>
          </div>
        </div>
      </div>

      <div className="settings-card" style={{ marginTop: "1rem" }}>
        <div className="settings-card-header settings-card-header-first">
          <Keyboard size={16} />
          <span>{t("settings.behavior", lang)}</span>
        </div>
        <div className="settings-row">
          <div className="settings-row-label">
            <div className="row-title">{t("settings.ender_char", lang)}</div>
            <div className="row-desc">{t("settings.ender_char_desc", lang)}</div>
          </div>
          <input
            type="text"
            maxLength={1}
            value={enderChar}
            onChange={handleEnderChange}
            className="settings-text-input"
          />
        </div>
        <div className="settings-row settings-row-last">
          <div className="settings-row-label">
            <div className="row-title">{t("settings.language", lang)}</div>
            <div className="row-desc">{t("settings.language_desc", lang)}</div>
          </div>
          <LanguageSelect value={lang} onChange={handleLangChange} />
        </div>
      </div>

      <div className="settings-card" style={{ marginTop: "1rem" }}>
        <div className="settings-card-header settings-card-header-first">
          <RefreshCw size={16} />
          <span>{t("settings.updates", lang)}</span>
        </div>
        <div className="settings-row">
          <div className="settings-row-label">
            <div className="row-title">{t("settings.check_updates", lang)}</div>
            <div className="row-desc">{t("settings.check_updates_desc", lang)}</div>
          </div>
          <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
            {updateStatus === "idle" && (
              <button className="btn-secondary" onClick={checkForUpdates} style={{ flexShrink: 0 }}>
                <RefreshCw size={14} />
                {t("settings.check", lang)}
              </button>
            )}
            {updateStatus === "checking" && (
              <span style={{ display: "flex", alignItems: "center", gap: "0.375rem", fontSize: "0.8125rem", color: "var(--text-muted)" }}>
                <Loader size={14} className="update-spinner" />
                {t("settings.checking", lang)}
              </span>
            )}
            {updateStatus === "uptodate" && (
              <span style={{ display: "flex", alignItems: "center", gap: "0.375rem", fontSize: "0.8125rem", color: "var(--success)", fontWeight: 500 }}>
                <Check size={14} />
                {t("settings.uptodate", lang)}
              </span>
            )}
            {updateStatus === "available" && availableUpdate && (
              <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
                <span style={{ fontSize: "0.8125rem", color: "var(--accent)", fontWeight: 500 }}>
                  v{availableUpdate.version}
                </span>
                <button
                  className="btn-primary"
                  onClick={handleInstall}
                  style={{ flexShrink: 0, padding: "0.375rem 0.75rem", fontSize: "0.75rem" }}
                >
                  <Download size={14} />
                  {t("settings.install", lang)}
                </button>
              </div>
            )}
            {updateStatus === "downloading" && (
              <span style={{ display: "flex", alignItems: "center", gap: "0.375rem", fontSize: "0.8125rem", color: "var(--text-muted)" }}>
                <Loader size={14} className="update-spinner" />
                {t("settings.downloading", lang)}
              </span>
            )}
            {updateStatus === "error" && (
              <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
                <span style={{ fontSize: "0.8125rem", color: "var(--danger)", fontWeight: 500 }}>
                  {t("settings.update_error", lang)}
                </span>
                <button className="btn-secondary" onClick={checkForUpdates} style={{ flexShrink: 0, padding: "0.375rem 0.75rem", fontSize: "0.75rem" }}>
                  <RefreshCw size={14} />
                  {t("settings.check_again", lang)}
                </button>
              </div>
            )}
          </div>
        </div>
        <div className="settings-row settings-row-last">
          <div className="settings-row-label">
            <div className="row-title">Current Version</div>
            <div className="row-desc">{currentVersion}</div>
          </div>
        </div>
      </div>
    </div>
  );
}

function LanguageSelect({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, []);

  const current = languages.find((l) => l.code === value) || languages[0];

  return (
    <div className="lang-select-wrapper" ref={ref}>
      <button className="lang-select" onClick={() => setOpen(!open)}>
        {current.label}
        <ChevronDown size={14} className={`lang-arrow ${open ? "open" : ""}`} />
      </button>
      {open && (
        <div className="lang-dropdown">
          {languages.map((l) => (
            <button
              key={l.code}
              className={`lang-option ${l.code === value ? "selected" : ""}`}
              onMouseDown={(e) => { e.preventDefault(); onChange(l.code); setOpen(false); }}
            >
              <span>{l.label}</span>
              {l.code === value && <Check size={14} />}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
