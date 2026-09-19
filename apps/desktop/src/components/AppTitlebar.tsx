import { WindowControls } from "./WindowControls";

const appIconUrl = new URL("../../app-icon.svg", import.meta.url).href;

export function AppTitlebar() {
  return (
    <header className="app-titlebar" data-tauri-drag-region="">
      <div className="app-brand" data-tauri-drag-region="">
        <img src={appIconUrl} width="24" height="24" alt="WhatsVault icon" />
        <span data-tauri-drag-region="">WhatsVault</span>
      </div>
      <WindowControls />
    </header>
  );
}
