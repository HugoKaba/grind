from rest_framework import permissions


class IsAuthorOrReadOnly(permissions.BasePermission):
    """Permission pour modifier son propre contenu ou lire."""

    def has_object_permission(self, request, view, obj):
        if request.method in permissions.SAFE_METHODS:
            return True
        return obj.author == request.user


class IsAuthor(permissions.BasePermission):
    """Permission pour modifier uniquement par l'auteur."""

    def has_object_permission(self, request, view, obj):
        return obj.author == request.user


class IsAdminOrReadOnly(permissions.BasePermission):
    """Permission pour admin ou lecture seule."""

    def has_permission(self, request, view):
        if request.method in permissions.SAFE_METHODS:
            return True
        return request.user and request.user.is_staff


class CanCreateQuestionComment(permissions.BasePermission):
    """Permission pour commenter sa propre question/réponse."""

    def has_permission(self, request, view):
        if request.method in permissions.SAFE_METHODS:
            return True
        return request.user and request.user.is_authenticated
