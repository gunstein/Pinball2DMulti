import {
  Container,
  Graphics,
  Text,
  TextStyle,
  TextStyleOptions,
} from "pixi.js";
import { CANVAS_WIDTH } from "../constants";
import { ConnectionState } from "../shared/ServerConnection";
import { Player } from "../shared/types";

/** Size of player indicator circles */
const PLAYER_DOT_RADIUS = 4;
/** Vertical spacing between player dots */
const PLAYER_DOT_SPACING = 12;
/** Maximum players to show before "..." */
const MAX_VISIBLE_PLAYERS = 20;

export class UILayer {
  container: Container;
  private hitCountText: Text;
  private hitCount = 0;
  private connectionText: Text;
  private playersContainer: Container;
  private playerDots: Graphics[] = [];

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

    // Players container (along right edge of board)
    this.playersContainer = new Container();
    this.playersContainer.x = CANVAS_WIDTH - 12; // Right edge of board area
    this.playersContainer.y = 50; // Start below top of board
    this.container.addChild(this.playersContainer);

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

  /** Update the connected players display */
  setPlayers(players: Player[], selfId: number) {
    // Clear existing dots
    for (const dot of this.playerDots) {
      dot.destroy();
    }
    this.playerDots = [];
    this.playersContainer.removeChildren();

    // Sort players: self first, then others
    const sortedPlayers = [...players].sort((a, b) => {
      if (a.id === selfId) return -1;
      if (b.id === selfId) return 1;
      return a.id - b.id;
    });

    // Limit visible players
    const hasMore = sortedPlayers.length > MAX_VISIBLE_PLAYERS;
    const visiblePlayers = hasMore
      ? sortedPlayers.slice(0, MAX_VISIBLE_PLAYERS)
      : sortedPlayers;

    // Create a dot for each player in a vertical column
    for (let i = 0; i < visiblePlayers.length; i++) {
      const player = visiblePlayers[i];

      const dot = new Graphics();
      dot.circle(0, 0, PLAYER_DOT_RADIUS);
      dot.fill({ color: player.color, alpha: 0.9 });

      // Add a ring around self
      if (player.id === selfId) {
        dot.circle(0, 0, PLAYER_DOT_RADIUS + 2);
        dot.stroke({ color: 0xffffff, width: 1, alpha: 0.7 });
      }

      dot.x = 0;
      dot.y = i * PLAYER_DOT_SPACING;

      this.playersContainer.addChild(dot);
      this.playerDots.push(dot);
    }

    // Show "..." and total count if there are more players
    if (hasMore) {
      const ellipsis = new Graphics();
      const y = MAX_VISIBLE_PLAYERS * PLAYER_DOT_SPACING;
      for (let i = 0; i < 3; i++) {
        ellipsis.circle(0, y + i * 6, 2);
        ellipsis.fill({ color: 0x888888, alpha: 0.7 });
      }
      this.playersContainer.addChild(ellipsis);
      this.playerDots.push(ellipsis);

      // Show total count
      const countText = new Text({
        text: `${sortedPlayers.length}`,
        style: {
          fontFamily: "monospace",
          fontSize: 10,
          fill: 0x888888,
        } as TextStyleOptions,
      });
      countText.anchor.set(0.5, 0);
      countText.x = 0;
      countText.y = y + 22;
      this.playersContainer.addChild(countText);
    }
  }
}
