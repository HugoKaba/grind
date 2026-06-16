from django.urls import path, include
from rest_framework.routers import DefaultRouter
from rest_framework_simplejwt.views import TokenObtainPairView, TokenRefreshView

from .viewsets import (
    TweetViewSet,
    LikeViewSet,
    RetweetViewSet,
    FollowViewSet,
    ProfileViewSet,
    HashtagViewSet,
)

router = DefaultRouter()
router.register(r'tweets', TweetViewSet, basename='tweet')
router.register(r'likes', LikeViewSet, basename='like')
router.register(r'retweets', RetweetViewSet, basename='retweet')
router.register(r'follows', FollowViewSet, basename='follow')
router.register(r'profiles', ProfileViewSet, basename='profile')
router.register(r'hashtags', HashtagViewSet, basename='hashtag')

urlpatterns = [
    path('', include(router.urls)),
    path('auth/token/', TokenObtainPairView.as_view(), name='token_obtain_pair'),
    path('auth/token/refresh/', TokenRefreshView.as_view(), name='token_refresh'),
]
