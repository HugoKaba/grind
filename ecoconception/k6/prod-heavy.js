// Charge LOURDE prod↔prod : monte à 100 VUs pour saturer le CPU (pas seulement la latence).
// Révèle l'écart d'efficacité serveur entre Rust et Django sous stress réel.
import http from "k6/http"; import { check } from "k6";
const BASE=__ENV.BASE||"https://grind-web.onrender.com"; const NAME=__ENV.NAME||"grind-heavy";
export const options={ cloud:{projectID:8038849,name:NAME},
  scenarios:{stress:{executor:"ramping-vus",startVUs:0,
    stages:[{duration:"15s",target:50},{duration:"20s",target:100},{duration:"30s",target:100},{duration:"10s",target:0}],
    gracefulStop:"5s"}},
  thresholds:{http_req_failed:["rate<0.10"]} };
export default function(){ const r=http.get(`${BASE}/`,{tags:{t:NAME}});
  check(r,{"200":x=>x.status===200}); }
