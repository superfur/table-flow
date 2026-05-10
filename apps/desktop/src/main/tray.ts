import { app, Menu, Tray, nativeImage, BrowserWindow } from "electron";

export function createTray(): Tray {
  const icon = nativeImage.createEmpty();
  const tray = new Tray(icon);

  const contextMenu = Menu.buildFromTemplate([
    { label: "TableFlow", enabled: false },
    { type: "separator" },
    {
      label: "Show",
      click: () => {
        const wins = BrowserWindow.getAllWindows();
        if (wins.length > 0) wins[0].show();
      },
    },
    { type: "separator" },
    {
      label: "Quit",
      click: () => {
        app.quit();
      },
    },
  ]);

  tray.setToolTip("TableFlow");
  tray.setContextMenu(contextMenu);

  return tray;
}
