# 🏋️ GRIND - Routes Complètes & Vérifiées

**Status: ✅ Tous les endpoints testés et fonctionnels**

---

## 📍 Base URLs

```
Web:     http://localhost:8000
API:     http://localhost:8000/api
Admin:   http://localhost:8000/admin
Swagger: http://localhost:8000/api/swagger/
```

---

## 🌐 WEB ROUTES (HTML Templates)

### Authentication
```
GET  /accounts/login/          → Login page (form)
GET  /logout/                  → Déconnexion (redirect login)
```

### Timeline & Posts
```
GET  /                         → Home timeline (2 onglets)
POST /tweet/post/              → Créer un post (form)
GET  /tweet/<id>/              → Détail d'un post
POST /tweet/<id>/like/         → Liker (form action)
POST /tweet/<id>/unlike/       → Déliker (form action)
POST /tweet/<id>/retweet/      → Retweet (form action)
POST /tweet/<id>/unretweet/    → Un-retweet (form action)
POST /tweet/<id>/reply/        → Répondre (form)
POST /tweet/<id>/bookmark/     → Bookmark (form action)
POST /tweet/<id>/unbookmark/   → Unbookmark (form action)
```

### User Profiles
```
GET  /@<username>/             → Profil utilisateur
POST /@<username>/follow/      → Suivre (form action)
POST /@<username>/unfollow/    → Arrêter de suivre (form action)
```

### Messaging
```
GET  /messages/                → Inbox (liste conversations)
GET  /messages/new/            → Créer conversation (user select)
GET  /messages/<username>/     → Thread DM avec user
POST /messages/<username>/     → Envoyer message (form)
```

### Discovery
```
GET  /notifications/           → Feed d'activité
GET  /trending/                → Hashtags tendance
GET  /bookmarks/               → Posts sauvegardés
```

### Documentation
```
GET  /api-docs/                → Documentation API custom
GET  /api/swagger/             → Swagger UI interactif ⭐
GET  /api/redoc/               → ReDoc (alternative)
```

### Admin
```
GET  /admin/                   → Django admin panel
```

---

## 🔌 API REST Routes (JSON)

### Authentication
```
POST /api/auth/token/          → Obtenir JWT token
  Body: {"username": "messi", "password": "messi123"}
  
POST /api/auth/token/refresh/  → Rafraîchir token expiré
  Body: {"refresh": "token..."}
```

### Tweets (Tous les CRUD)
```
GET  /api/tweets/              → Lister tous tweets (paginated)
POST /api/tweets/              → Créer tweet (auth required)
  Body: {"content": "Amazing goal!"}

GET  /api/tweets/{id}/         → Détail tweet
PUT  /api/tweets/{id}/         → Modifier tweet (author only)
DELETE /api/tweets/{id}/       → Supprimer tweet (author only)

POST /api/tweets/{id}/like/    → Liker
GET  /api/tweets/?search=goal  → Chercher tweets
```

### Likes
```
GET  /api/likes/               → Lister likes
POST /api/likes/               → Créer like
GET  /api/likes/{id}/          → Like detail
DELETE /api/likes/{id}/        → Supprimer like
```

### Retweets
```
GET  /api/retweets/            → Lister retweets
POST /api/retweets/            → Créer retweet
DELETE /api/retweets/{id}/     → Supprimer retweet
```

### Follows
```
GET  /api/follows/             → Lister follows
POST /api/follows/             → Créer follow (user A suit user B)
  Body: {"follower": 1, "following": 2}

DELETE /api/follows/{id}/      → Arrêter follow
```

### Profiles
```
GET  /api/profiles/            → Lister profiles
GET  /api/profiles/{id}/       → Profile detail
PUT  /api/profiles/{id}/       → Modifier profile (owner only)
```

### Hashtags
```
GET  /api/hashtags/            → Lister hashtags
GET  /api/hashtags/{id}/       → Hashtag detail + tweets
```

### Messages (DMs)
```
GET  /api/messages/            → Lister conversations
POST /api/messages/            → Envoyer message
GET  /api/messages/{id}/       → Détail message
```

### Browsable API
```
GET  /api/                     → API root page (browse all)
GET  /api/auth/                → Auth endpoints
```

### Schema
```
GET  /api/schema/              → OpenAPI schema JSON
```

---

## 📊 HTTP Methods Expliqués

| Method | Action | Exemple |
|--------|--------|---------|
| **GET** | Récupérer données | `GET /tweets/1/` - voir un tweet |
| **POST** | Créer ressource | `POST /tweets/` - créer tweet |
| **PUT** | Modifier complet | `PUT /tweets/1/` - modifier tout |
| **PATCH** | Modifier partiel | `PATCH /tweets/1/` - modifier un champ |
| **DELETE** | Supprimer | `DELETE /tweets/1/` - supprimer |

---

## 🔐 Authentication

### Pour Web (Django Session)
```
1. POST /accounts/login/
   - Form: username + password
   - Django crée session
   - Cookie sauvé dans navigateur

2. GET / (avec cookie)
   - Django lit session
   - user.is_authenticated = True
```

### Pour API (JWT Token)
```
1. POST /api/auth/token/
   - Body: {"username": "messi", "password": "messi123"}
   - Response: {"access": "token...", "refresh": "token..."}

2. GET /api/tweets/ (avec token)
   - Header: Authorization: Bearer token...
   - API valide token
```

---

## ✅ Pages Testées

- ✅ **Home** `/` - Timeline avec 2 onglets
- ✅ **Login** `/accounts/login/` - Formulaire auth
- ✅ **Logout** `/logout/` - Déconnexion
- ✅ **Profiles** `/@messi/` - Profil user
- ✅ **Tweets** `/tweet/1/` - Détail post
- ✅ **Messages** `/messages/` - Inbox
- ✅ **Notifications** `/notifications/` - Feed activité
- ✅ **Trending** `/trending/` - Hashtags
- ✅ **Bookmarks** `/bookmarks/` - Posts sauvés
- ✅ **Swagger** `/api/swagger/` - API docs interactif ⭐
- ✅ **API Docs** `/api-docs/` - Documentation custom

---

## 🚀 Toutes les Actions Testées

### Posts
- ✅ Create (POST /tweet/post/)
- ✅ Like/Unlike (POST /tweet/{id}/like/)
- ✅ Retweet/Un-retweet (POST /tweet/{id}/retweet/)
- ✅ Reply (POST /tweet/{id}/reply/)
- ✅ Bookmark (POST /tweet/{id}/bookmark/)

### Users
- ✅ Follow/Unfollow (POST /@{username}/follow/)
- ✅ View Profile (GET /@{username}/)
- ✅ Login/Logout (POST /accounts/login/, GET /logout/)

### Messages
- ✅ List conversations (GET /messages/)
- ✅ Send message (POST /messages/{username}/)
- ✅ View thread (GET /messages/{username}/)

### API
- ✅ Get tweets (GET /api/tweets/)
- ✅ Create tweet (POST /api/tweets/)
- ✅ JWT auth (POST /api/auth/token/)
- ✅ Pagination (GET /api/tweets/?page=2)

---

## 📋 Test Credentials (Vérifiés ✅)

```
Username: messi        Password: messi123
Username: ronaldo      Password: ronaldo123
Username: neymar       Password: neymar123
Username: mbappe       Password: mbappe123
Username: haaland      Password: haaland123
Username: benzema      Password: benzema123
```

---

## 🎯 Swagger Interactif

**L'une des meilleures features!** 

👉 **http://localhost:8000/api/swagger/**

Ici tu peux:
- ✅ **Voir tous les endpoints** (auto-documenté)
- ✅ **Tester les routes directement** (Try it out!)
- ✅ **Voir les réponses JSON** en temps réel
- ✅ **Authentifier** avec JWT token
- ✅ **Voir le schéma OpenAPI** complet

---

## 🔒 Security Features

- ✅ CSRF Protection (tous les forms)
- ✅ JWT Authentication (API)
- ✅ Session Auth (Web)
- ✅ Login Required (certains endpoints)
- ✅ Permissions (owner only pour delete/edit)
- ✅ XSS Prevention (template escaping)
- ✅ SQL Injection Prevention (ORM)

---

## 📊 Statistiques

```
Total Routes:      30+
Web Routes:        20+
API Routes:        15+
Auth Methods:      2 (Session + JWT)
Models:            8 (Tweet, Like, Follow, Message, etc.)
Views:             15+ (web + API)
Templates:         10+
Test Credentials:  6 athletes
```

---

## 🎉 Résumé Final

| Aspect | Status | Notes |
|--------|--------|-------|
| **Home Page** | ✅ | 2 onglets, AJAX actions |
| **Profiles** | ✅ | Follow/unfollow, stats |
| **Messages** | ✅ | DMs, conversations |
| **Notifications** | ✅ | Activity feed |
| **Login/Logout** | ✅ | Sessions Django |
| **API** | ✅ | 15+ endpoints DRF |
| **Swagger UI** | ✅ | Interactif, testable |
| **Documentation** | ✅ | Routes complètes ici |
| **Real Data** | ✅ | 6 athlètes, 40+ posts |
| **Responsive Design** | ✅ | Mobile/tablet/desktop |

---

## 🚀 Ready for Demo!

**Tout est testé, documenté et prêt pour la présentation!** 🎓

- Code clean et commenté
- Swagger interactif pour tester
- Vraies données (athletes, posts)
- Toutes les routes documentées
- Responsive design

**C'est du vrai Django fullstack!** 💪

---

*Dernière vérification: June 18, 2026*  
*Tous les endpoints testés et fonctionnels ✅*
