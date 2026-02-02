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
const PLAYER_DOT_SPACING = 16;
/** Maximum players to show before "..." */
const MAX_VISIBLE_PLAYERS = 20;

/** Connection status colors */
const CONNECTION_COLORS = {
  connected: 0x44ff44, // Green
  connecting: 0xffaa00, // Yellow/Orange
  disconnected: 0xff4444, // Red
};

export class UILayer {
  container: Container;
  private hitCountText: Text;
  private hitCount = 0;
  private connectionText: Text;
  private connectionDot: Graphics;
  private playersContainer: Container;
  private playerDots: Graphics[] = [];
  private playerTexts: Text[] = [];
  private playerSummaryText: Text;

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
    // Dot at x=0, text extends to right - position so text ends at right edge
    this.playersContainer = new Container();
    this.playersContainer.x = CANVAS_WIDTH - 12; // Dot position, text extends to right
    this.playersContainer.y = 70; // Start below summary text with more space
    this.container.addChild(this.playersContainer);

    // Player summary text (active/total) - right-aligned to edge
    this.playerSummaryText = new Text({
      text: "",
      style: {
        fontFamily: "monospace",
        fontSize: 10,
        fill: 0x888888,
      } as TextStyleOptions,
    });
    this.playerSummaryText.anchor.set(1, 0); // Right-align
    this.playerSummaryText.x = CANVAS_WIDTH - 2;
    this.playerSummaryText.y = 35;
    this.container.addChild(this.playerSummaryText);

    // Connection status dot (top-left corner)
    this.connectionDot = new Graphics();
    this.connectionDot.x = 35;
    this.connectionDot.y = 45;
    this.drawConnectionDot(CONNECTION_COLORS.connecting);
    this.container.addChild(this.connectionDot);

    // Connection status text (next to dot)
    const connectionStyle = new TextStyle({
      fontFamily: "monospace",
      fontSize: 12,
      fill: 0xffaa00,
      align: "left",
    });
    this.connectionText = new Text({ text: "", style: connectionStyle });
    this.connectionText.x = 47;
    this.connectionText.y = 40;
    this.connectionText.visible = false;
    this.container.addChild(this.connectionText);
  }

  private drawConnectionDot(color: number) {
    this.connectionDot.clear();
    // Outer glow
    this.connectionDot.circle(0, 0, 8);
    this.connectionDot.fill({ color, alpha: 0.3 });
    // Inner core
    this.connectionDot.circle(0, 0, 5);
    this.connectionDot.fill({ color, alpha: 0.9 });
  }

  addHit() {
    this.hitCount++;
    this.hitCountText.text = `Hits: ${this.hitCount}`;
  }

  setConnectionState(state: ConnectionState) {
    const color = CONNECTION_COLORS[state];
    this.drawConnectionDot(color);

    switch (state) {
      case "connected":
        this.connectionText.visible = false;
        break;
      case "connecting":
        this.connectionText.text = "Connecting...";
        this.connectionText.style.fill = color;
        this.connectionText.visible = true;
        break;
      case "disconnected":
        this.connectionText.text = "Offline";
        this.connectionText.style.fill = color;
        this.connectionText.visible = true;
        break;
    }
  }

  /** Update the connected players display */
  setPlayers(players: Player[], selfId: number) {
    // Clear existing dots and texts
    for (const dot of this.playerDots) {
      dot.destroy();
    }
    for (const text of this.playerTexts) {
      text.destroy();
    }
    this.playerDots = [];
    this.playerTexts = [];
    this.playersContainer.removeChildren();

    // Sort players: self first, then others
    const sortedPlayers = [...players].sort((a, b) => {
      if (a.id === selfId) return -1;
      if (b.id === selfId) return 1;
      return a.id - b.id;
    });

    // Count active players
    const activePlayers = players.filter((p) => !p.paused).length;
    this.playerSummaryText.text = `${activePlayers}/${players.length} active`;

    // Limit visible players
    const hasMore = sortedPlayers.length > MAX_VISIBLE_PLAYERS;
    const visiblePlayers = hasMore
      ? sortedPlayers.slice(0, MAX_VISIBLE_PLAYERS)
      : sortedPlayers;

    // Create a dot and stats for each player in a vertical column
    for (let i = 0; i < visiblePlayers.length; i++) {
      const player = visiblePlayers[i];

      const dot = new Graphics();
      // Paused players are semi-transparent
      const alpha = player.paused ? 0.3 : 0.9;
      dot.circle(0, 0, PLAYER_DOT_RADIUS);
      dot.fill({ color: player.color, alpha });

      // Add a ring around self
      if (player.id === selfId) {
        dot.circle(0, 0, PLAYER_DOT_RADIUS + 2);
        dot.stroke({
          color: 0xffffff,
          width: 1,
          alpha: player.paused ? 0.2 : 0.7,
        });
      }

      dot.x = 0;
      dot.y = i * PLAYER_DOT_SPACING;

      this.playersContainer.addChild(dot);
      this.playerDots.push(dot);

      // Add stats text to right of dot: "ballsInFlight / ballsProduced"
      const statsText = new Text({
        text: `${player.ballsInFlight}/${player.ballsProduced}`,
        style: {
          fontFamily: "monospace",
          fontSize: 10,
          fill: player.paused ? 0x666666 : 0xaaaaaa,
        } as TextStyleOptions,
      });
      statsText.anchor.set(0, 0.5); // Left-align (text to right of dot)
      statsText.x = PLAYER_DOT_RADIUS + 6;
      statsText.y = i * PLAYER_DOT_SPACING;

      this.playersContainer.addChild(statsText);
      this.playerTexts.push(statsText);
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
