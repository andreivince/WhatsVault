export interface LatestRequestGate {
  begin(): () => boolean;
  invalidate(): void;
}

export function createLatestRequestGate(): LatestRequestGate {
  let activeRequest = Symbol("initial request");

  return {
    begin() {
      const request = Symbol("request");
      activeRequest = request;
      return () => activeRequest === request;
    },
    invalidate() {
      activeRequest = Symbol("invalidated request");
    },
  };
}
