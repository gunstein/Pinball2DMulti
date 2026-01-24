import { Container, Text, TextStyle } from "pixi.js";
import { CANVAS_WIDTH } from "../constants";

export class UILayer {
  container: Container;
  private hitCountText: Text;
  private hitCount = 0;

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
  }

  addHit() {
    this.hitCount++;
    this.hitCountText.text = `Hits: ${this.hitCount}`;
  }
}
