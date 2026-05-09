// TODO(detail-impl): HUD 主容器
import type { Component } from "solid-js";

export interface HudOverlayProps {
  tableId: string;
}

export const HudOverlay: Component<HudOverlayProps> = (_props) => {
  return <div class="overlay-container" />;
};
