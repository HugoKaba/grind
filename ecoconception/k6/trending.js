// Test de charge k6 — route publique /trending (rendue en SSR).
// Le rendu SSR appelle la server function `get_trending` :
//   - sans Redis (NoopCache)  -> recalcul agrégé en BDD à CHAQUE requête (énergivore)
//   - avec Redis (RedisCache) -> servi du cache (HIT), la BDD n'est plus touchée
// On lance EXACTEMENT le même test dans les deux cas pour comparer (avant/après).
//
//   k6 run ecoconception/k6/trending.js
//   BASE=http://127.0.0.1:3000 k6 run --summary-export=out.json ecoconception/k6/trending.js
import http from "k6/http";
import { check } from "k6";

export const options = {
  scenarios: {
    charge: {
      executor: "constant-vus",
      vus: 30,
      duration: "20s",
    },
  },
  thresholds: {
    http_req_failed: ["rate<0.01"],
    http_req_duration: ["p(95)<500"],
  },
};

const BASE = __ENV.BASE || "http://127.0.0.1:3000";

export default function () {
  const res = http.get(`${BASE}/trending`);
  check(res, {
    "status 200": (r) => r.status === 200,
    "contient Trending": (r) => r.body && r.body.includes("Trending"),
  });
}
