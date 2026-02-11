export interface LocationLike {
  protocol: string;
  host: string;
}

export function buildServerUrl(
  locationLike: LocationLike,
  envOverride?: string,
): string {
  if (envOverride && envOverride.length > 0) {
    return envOverride;
  }
  const wsScheme = locationLike.protocol === "https:" ? "wss" : "ws";
  return `${wsScheme}://${locationLike.host}/ws`;
}

export function launcherStackScale(count: number): number {
  const c = Math.max(1, count);
  return c * c;
}
