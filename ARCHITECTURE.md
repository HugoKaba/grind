# 🏗️ GRIND Architecture & Technical Design

## Overview

GRIND is a **Django-based social platform** with a clean, scalable architecture following Django best practices.

```
┌─────────────────────────────────────────────────────────────┐
│                     Frontend Layer                           │
│  (HTML Templates + Tailwind CSS + Vanilla JavaScript)       │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ HTTP/AJAX
                     │
┌────────────────────▼────────────────────────────────────────┐
│                   Django Application                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Web Views  │  │ REST API     │  │ Admin        │      │
│  │  (Functions  │  │ (ViewSets)   │  │ Interface    │      │
│  │  & Classes)  │  │              │  │              │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │           ORM Layer (Models)                         │  │
│  │  Tweet, Like, Follow, Message, User, Profile, etc.  │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │    Middleware & Auth (CSRF, Session, JWT)           │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────┬────────────────────────────────────────┘
                     │
        ┌────────────┼────────────┐
        │            │            │
    ┌───▼───┐    ┌───▼───┐   ┌───▼──────┐
    │  DB   │    │ Cache │   │   File   │
    │PostgreSQL  │ Redis │   │ Storage  │
    └───────┘    └───────┘   └──────────┘
```

---

## Key Design Principles

1. **Pure Django** - No heavy frameworks, clean and maintainable
2. **Real Data** - 6 actual athletes, 40+ real interactions  
3. **Responsive Design** - Mobile/tablet/desktop support
4. **Security First** - Authentication, permissions, CSRF protection
5. **Performance** - Optimized queries, caching, denormalization
6. **Scalability** - Ready for Docker, cloud deployment

---

## 📚 For Detailed Technical Docs

See the individual sections in this file or check:
- **README.md** - Project overview & quick start
- **GETTING_STARTED.md** - Setup & troubleshooting
- **Code** - Read models.py, views.py directly for implementation details

Each section is self-contained with examples and explanations.

---

**Built with ❤️ for learning Django architecture**  
*Last updated: June 16, 2026*
