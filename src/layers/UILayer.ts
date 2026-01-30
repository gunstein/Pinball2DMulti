import { Container, Text, TextStyle } from "pixi.js";
import { CANVAS_WIDTH } from "../constants";
import { ConnectionState } from "../shared/ServerConnection";

export class UILayer {
  container: Container;
  private hitCountText: Text;
  private hitCount = 0;
  private connectionText: Text;

  constructor() {
    this.container = new Container();

    const style = new TextStyle({
      fontFamily: "monospace",
      fontSize: 16,
      fill: 0x4da6a6,
      align: "right",
    });

    this.hitCountText = new Text({ text: "Hits: 0", style });
    this.hitCountText.x = CANVAS_WIDTH - 100;
    this.hitCountText.y = 10;
    this.container.addChild(this.hitCountText);

    // Connection status indicator
    const connectionStyle = new TextStyle({
      fontFamily: "monospace",
      fontSize: 12,
      fill: 0xffaa00,
      align: "left",
    });
    this.connectionText = new Text({ text: "", style: connectionStyle });
    this.connectionText.x = 10;
    this.connectionText.y = 10;
    this.connectionText.visible = false;
    this.container.addChild(this.connectionText);
  }

  addHit() {
    this.hitCount++;
    this.hitCountText.text = `Hits: ${this.hitCount}`;
  }

  setConnectionState(state: ConnectionState) {
    switch (state) {
      case "connected":
        this.connectionText.visible = false;
        break;
      case "connecting":
        this.connectionText.text = "Connecting...";
        this.connectionText.style.fill = 0xffaa00;
        this.connectionText.visible = true;
        break;
      case "disconnected":
        this.connectionText.text = "Offline - reconnecting...";
        this.connectionText.style.fill = 0xff6666;
        this.connectionText.visible = true;
        break;
    }
  }
}
