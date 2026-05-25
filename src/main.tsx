import React from "react";
import ReactDOM from "react-dom/client";
import Overlay from "./Overlay";
import Replay from "./Replay";
import Settings from "./Settings";
import TileWindow from "./TileWindow";
import "./styles.css";

const params = new URLSearchParams(window.location.search);
const isSettings = params.get("settings") === "1";
const isTile = params.get("tile") === "1";
const isReplay = params.get("replay") === "1";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {isTile ? (
      <TileWindow />
    ) : isReplay ? (
      <Replay />
    ) : isSettings ? (
      <Settings />
    ) : (
      <Overlay />
    )}
  </React.StrictMode>
);
