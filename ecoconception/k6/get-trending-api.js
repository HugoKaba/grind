// Test de charge k6 sur la FONCTION énergivore `get_trending` (agrégation des
// hashtags tendance). On appelle directement l'endpoint server-function JSON.
//
//   AVANT (sans cache) : serveur lancé SANS REDIS_URL -> NoopCache
//                        -> chaque requête recalcule en BDD (MISS permanent).
//   APRÈS (avec cache) : serveur lancé AVEC REDIS_URL -> RedisCache
//                        -> quasi toutes les requêtes servies du cache (HIT).
//
// Lancer le MÊME test dans les deux cas et comparer p95 / req·s :
//   k6 run --summary-export=ecoconception/k6/summary-nocache.json ecoconception/k6/get-trending-api.js
//   k6 run --summary-export=ecoconception/k6/summary-cache.json   ecoconception/k6/get-trending-api.js
import http from "k6/http";
import { check } from "k6";
import { Counter } from "k6/metrics";

const hits = new Counter("x_cache_hit");
const misses = new Counter("x_cache_miss");

export const options = {
  scenarios: {
    charge: { executor: "constant-vus", vus: 40, duration: "20s" },
  },
  thresholds: {
    http_req_failed: ["rate<0.01"],
  },
};

const BASE = __ENV.BASE || "http://127.0.0.1:3000";

export default function () {
  const res = http.post(`${BASE}/api/get_trending`, null, {
    headers: { "content-type": "application/x-www-form-urlencoded" },
  });
  check(res, { "status 200": (r) => r.status === 200 });
  const xc = res.headers["X-Cache"];
  if (xc === "HIT") hits.add(1);
  else if (xc === "MISS") misses.add(1);
}
