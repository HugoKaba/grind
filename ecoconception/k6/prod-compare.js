// Comparaison de charge PROD ↔ PROD (écoconception) : même test, deux serveurs
// hébergés gratuitement sur Render, sur la route commune la plus « énergivore » :
// la page d'accueil `/` (feed) rendue côté serveur (requête BDD + rendu HTML).
//
//   Rust/Leptos+Axum : https://grind-web.onrender.com
//   Django (baseline) : https://grind-django.onrender.com
//
// Lancer dans le cloud k6 (résultats groupés dans le projet grind-loadtest) :
//   BASE=https://grind-django.onrender.com NAME=grind-prod-django k6 cloud run ecoconception/k6/prod-compare.js
//   BASE=https://grind-web.onrender.com    NAME=grind-prod-rust   k6 cloud run ecoconception/k6/prod-compare.js
import http from "k6/http";
import { check } from "k6";

const BASE = __ENV.BASE || "https://grind-web.onrender.com";
const NAME = __ENV.NAME || "grind-prod";

export const options = {
  cloud: {
    projectID: 8038849, // projet "grind-loadtest"
    name: NAME,
  },
  // Montée en charge identique pour les deux serveurs : 0→30 VUs puis palier.
  scenarios: {
    charge: {
      executor: "ramping-vus",
      startVUs: 0,
      stages: [
        { duration: "20s", target: 30 },
        { duration: "40s", target: 30 },
        { duration: "10s", target: 0 },
      ],
      gracefulStop: "5s",
    },
  },
  thresholds: {
    http_req_failed: ["rate<0.05"],
    http_req_duration: ["p(95)<3000"],
  },
};

export default function () {
  const res = http.get(`${BASE}/`, { tags: { target: NAME } });
  check(res, {
    "status 200": (r) => r.status === 200,
    "html feed": (r) => !!r.body && r.body.includes("GRIND"),
  });
}
