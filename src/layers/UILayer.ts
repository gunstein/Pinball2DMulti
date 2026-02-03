import { Container, Graphics, Text, TextStyleOptions } from "pixi.js";
import { CANVAS_WIDTH } from "../constants";
import { ConnectionState } from "../shared/ServerConnection";
import { Player } from "../shared/types";

/** Size of player indicator circles */
const PLAYER_DOT_RADIUS = 4;
/** Vertical spacing between player dots */
const PLAYER_DOT_SPACING = 16;
/** Maximum players to show before "..." */
const MAX_VISIBLE_PLAYERS = 20;

/** Icon sizes (relative to canvas, scales with resolution) */
const ICON_SIZE = 12;
const ICON_SMALL = 8;

/** Connection status colors */
const CONNECTION_COLORS = {
  connected: 0x44ff44, // Green
  connecting: 0xffaa00, // Yellow/Orange
  disconnected: 0xff4444, // Red
};

/** UI colors */
const UI_COLOR = 0x4da6a6;
const UI_COLOR_DIM = 0x888888;

export class UILayer {
  container: Container;
  private hitIcon: Graphics;
  private hitCountText: Text;
  private hitCount = 0;
  private connectionDot: Graphics;
  private playersIcon: Graphics;
  private playersSummaryText: Text;
  private playersContainer: Container;
  private playerDots: Graphics[] = [];
  private playerTexts: Text[] = [];

  constructor() {
    this.container = new Container();

    // Hit counter icon (lightning bolt) + number
    this.hitIcon = new Graphics();
    this.hitIcon.x = CANVAS_WIDTH - 45;
    this.hitIcon.y = 12;
    this.drawLightningBolt(this.hitIcon, ICON_SIZE, UI_COLOR);
    this.container.addChild(this.hitIcon);

    this.hitCountText = new Text({
      text: "0",
      style: {
        fontFamily: "monospace",
        fontSize: 14,
        fill: UI_COLOR,
      } as TextStyleOptions,
    });
    this.hitCountText.anchor.set(0, 0);
    this.hitCountText.x = CANVAS_WIDTH - 30;
    this.hitCountText.y = 8;
    this.container.addChild(this.hitCountText);

    // Connection status dot (top-left corner)
    this.connectionDot = new Graphics();
    this.connectionDot.x = 35;
    this.connectionDot.y = 45;
    this.drawConnectionDot(CONNECTION_COLORS.connecting);
    this.container.addChild(this.connectionDot);

    // Players icon (person silhouette) + count
    this.playersIcon = new Graphics();
    this.playersIcon.x = CANVAS_WIDTH - 45;
    this.playersIcon.y = 38;
    this.drawPersonIcon(this.playersIcon, ICON_SMALL, UI_COLOR_DIM);
    this.container.addChild(this.playersIcon);

    this.playersSummaryText = new Text({
      text: "",
      style: {
        fontFamily: "monospace",
        fontSize: 10,
        fill: UI_COLOR_DIM,
      } as TextStyleOptions,
    });
    this.playersSummaryText.anchor.set(0, 0.5);
    this.playersSummaryText.x = CANVAS_WIDTH - 33;
    this.playersSummaryText.y = 42;
    this.container.addChild(this.playersSummaryText);

    // Players container (along right edge of board)
    this.playersContainer = new Container();
    this.playersContainer.x = CANVAS_WIDTH - 12;
    this.playersContainer.y = 60;
    this.container.addChild(this.playersContainer);
  }

  /** Draw a lightning bolt icon (for hits) */
  private drawLightningBolt(g: Graphics, size: number, color: number) {
    g.clear();
    const s = size;
    // Lightning bolt shape
    g.moveTo(s * 0.5, 0);
    g.lineTo(s * 0.1, s * 0.5);
    g.lineTo(s * 0.4, s * 0.5);
    g.lineTo(s * 0.2, s);
    g.lineTo(s * 0.9, s * 0.4);
    g.lineTo(s * 0.55, s * 0.4);
    g.lineTo(s * 0.8, 0);
    g.closePath();
    g.fill({ color, alpha: 0.9 });
  }

  /** Draw a person silhouette icon (for player count) */
  private drawPersonIcon(g: Graphics, size: number, color: number) {
    g.clear();
    const s = size;
    // Head
    g.circle(s * 0.5, s * 0.25, s * 0.2);
    g.fill({ color, alpha: 0.9 });
    // Body (simplified torso)
    g.roundRect(s * 0.2, s * 0.5, s * 0.6, s * 0.5, s * 0.1);
    g.fill({ color, alpha: 0.9 });
  }

  /** Draw a small ball icon (for ball stats) */
  private drawBallIcon(
    g: Graphics,
    x: number,
    y: number,
    size: number,
    color: number,
    filled: boolean,
  ) {
    g.circle(x, y, size);
    if (filled) {
      g.fill({ color, alpha: 0.8 });
    } else {
      g.stroke({ color, width: 1, alpha: 0.6 });
    }
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
    this.hitCountText.text = `${this.hitCount}`;
  }

  setConnectionState(state: ConnectionState) {
    const color = CONNECTION_COLORS[state];
    this.drawConnectionDot(color);
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
    this.playersSummaryText.text = `${activePlayers}/${players.length}`;

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

      // Draw ball stats as small circles: filled = in flight, outline = produced
      const ballSize = 2;
      const ballSpacing = 6;
      const statsX = PLAYER_DOT_RADIUS + 8;

      // Show balls in flight (filled) and additional produced (outline)
      // Format: [filled][filled][outline][outline] for 2 in flight, 4 total produced
      const maxBallsToShow = 5;
      const inFlight = Math.min(player.ballsInFlight, maxBallsToShow);
      const produced = Math.min(player.ballsProduced, maxBallsToShow);

      for (let b = 0; b < inFlight; b++) {
        this.drawBallIcon(
          dot,
          statsX + b * ballSpacing,
          0,
          ballSize,
          player.color,
          true,
        );
      }
      // Show remaining produced as outlines (if space)
      const remaining = Math.min(
        produced - inFlight,
        maxBallsToShow - inFlight,
      );
      for (let b = 0; b < remaining; b++) {
        this.drawBallIcon(
          dot,
          statsX + (inFlight + b) * ballSpacing,
          0,
          ballSize,
          player.color,
          false,
        );
      }
      // If more than maxBallsToShow, show "+" indicator
      if (player.ballsProduced > maxBallsToShow) {
        const plusX = statsX + maxBallsToShow * ballSpacing + 2;
        dot.moveTo(plusX, -2);
        dot.lineTo(plusX, 2);
        dot.moveTo(plusX - 2, 0);
        dot.lineTo(plusX + 2, 0);
        dot.stroke({ color: player.paused ? 0x666666 : 0xaaaaaa, width: 1 });
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

      // Show total count as number
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
      this.playerTexts.push(countText);
    }
  }
}
