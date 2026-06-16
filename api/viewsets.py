from rest_framework import viewsets, status, filters
from rest_framework.decorators import action
from rest_framework.response import Response
from rest_framework.permissions import IsAuthenticated, AllowAny
from django_filters.rest_framework import DjangoFilterBackend
from django.shortcuts import get_object_or_404
from django.contrib.auth.models import User
from django.db.models import Q

from core.models import Tweet, Like, Retweet, Follow, Hashtag, Profile
from .serializers import (
    TweetListSerializer,
    TweetDetailSerializer,
    TweetCreateSerializer,
    LikeSerializer,
    RetweetSerializer,
    FollowSerializer,
    ProfileSerializer,
    HashtagSerializer,
)
from .permissions import IsAuthorOrReadOnly, IsAuthor


class ProfileViewSet(viewsets.ReadOnlyModelViewSet):
    queryset = Profile.objects.all()
    serializer_class = ProfileSerializer
    permission_classes = [AllowAny]
    filter_backends = [filters.SearchFilter]
    search_fields = ['user__username', 'bio']


class TweetViewSet(viewsets.ModelViewSet):
    queryset = Tweet.objects.select_related('author').prefetch_related('liked_by', 'retweeted_by', 'replies')
    permission_classes = [IsAuthorOrReadOnly]
    filter_backends = [DjangoFilterBackend, filters.SearchFilter, filters.OrderingFilter]
    filterset_fields = ['author']
    search_fields = ['content', 'author__username']
    ordering_fields = ['created_at']
    ordering = ['-created_at']

    def get_serializer_class(self):
        if self.action == 'create':
            return TweetCreateSerializer
        elif self.action == 'retrieve':
            return TweetDetailSerializer
        return TweetListSerializer

    def get_permissions(self):
        if self.action in ['create', 'update', 'partial_update', 'destroy']:
            return [IsAuthenticated(), IsAuthor()]
        return [IsAuthorOrReadOnly()]

    def perform_create(self, serializer):
        serializer.save(author=self.request.user)

    @action(detail=True, methods=['post'], permission_classes=[IsAuthenticated])
    def like(self, request, pk=None):
        tweet = self.get_object()
        like, created = Like.objects.get_or_create(user=request.user, tweet=tweet)
        if created:
            tweet.likes_count += 1
            tweet.save(update_fields=['likes_count'])
        return Response({'status': 'liked'}, status=status.HTTP_200_OK)

    @action(detail=True, methods=['post'], permission_classes=[IsAuthenticated])
    def unlike(self, request, pk=None):
        tweet = self.get_object()
        deleted, _ = Like.objects.filter(user=request.user, tweet=tweet).delete()
        if deleted:
            tweet.likes_count = max(0, tweet.likes_count - 1)
            tweet.save(update_fields=['likes_count'])
        return Response({'status': 'unliked'}, status=status.HTTP_200_OK)

    @action(detail=True, methods=['post'], permission_classes=[IsAuthenticated])
    def retweet(self, request, pk=None):
        tweet = self.get_object()
        retweet, created = Retweet.objects.get_or_create(user=request.user, tweet=tweet)
        if created:
            tweet.retweets_count += 1
            tweet.save(update_fields=['retweets_count'])
        return Response({'status': 'retweeted'}, status=status.HTTP_200_OK)

    @action(detail=True, methods=['post'], permission_classes=[IsAuthenticated])
    def unretweet(self, request, pk=None):
        tweet = self.get_object()
        deleted, _ = Retweet.objects.filter(user=request.user, tweet=tweet).delete()
        if deleted:
            tweet.retweets_count = max(0, tweet.retweets_count - 1)
            tweet.save(update_fields=['retweets_count'])
        return Response({'status': 'unretweeted'}, status=status.HTTP_200_OK)

    @action(detail=True, methods=['post'], permission_classes=[IsAuthenticated])
    def reply(self, request, pk=None):
        parent_tweet = self.get_object()
        serializer = TweetCreateSerializer(data=request.data, context={'request': request})
        if serializer.is_valid():
            serializer.save(author=request.user, parent_tweet=parent_tweet)
            parent_tweet.replies_count += 1
            parent_tweet.save(update_fields=['replies_count'])
            return Response(serializer.data, status=status.HTTP_201_CREATED)
        return Response(serializer.errors, status=status.HTTP_400_BAD_REQUEST)


class LikeViewSet(viewsets.ModelViewSet):
    queryset = Like.objects.all()
    serializer_class = LikeSerializer
    permission_classes = [IsAuthorOrReadOnly]

    def get_permissions(self):
        if self.action in ['create', 'destroy']:
            return [IsAuthenticated(), IsAuthor()]
        return [IsAuthorOrReadOnly()]


class RetweetViewSet(viewsets.ModelViewSet):
    queryset = Retweet.objects.all()
    serializer_class = RetweetSerializer
    permission_classes = [IsAuthorOrReadOnly]

    def get_permissions(self):
        if self.action in ['create', 'destroy']:
            return [IsAuthenticated(), IsAuthor()]
        return [IsAuthorOrReadOnly()]


class FollowViewSet(viewsets.ViewSet):
    permission_classes = [IsAuthenticated]

    @action(detail=True, methods=['post'])
    def follow(self, request, pk=None):
        user_to_follow = get_object_or_404(User, pk=pk)
        if request.user == user_to_follow:
            return Response({'error': 'Cannot follow yourself'}, status=status.HTTP_400_BAD_REQUEST)

        follow, created = Follow.objects.get_or_create(follower=request.user, following=user_to_follow)
        if created:
            request.user.profile.following_count += 1
            user_to_follow.profile.followers_count += 1
            request.user.profile.save(update_fields=['following_count'])
            user_to_follow.profile.save(update_fields=['followers_count'])
        return Response({'status': 'followed'}, status=status.HTTP_200_OK)

    @action(detail=True, methods=['post'])
    def unfollow(self, request, pk=None):
        user_to_unfollow = get_object_or_404(User, pk=pk)
        deleted, _ = Follow.objects.filter(follower=request.user, following=user_to_unfollow).delete()
        if deleted:
            request.user.profile.following_count = max(0, request.user.profile.following_count - 1)
            user_to_unfollow.profile.followers_count = max(0, user_to_unfollow.profile.followers_count - 1)
            request.user.profile.save(update_fields=['following_count'])
            user_to_unfollow.profile.save(update_fields=['followers_count'])
        return Response({'status': 'unfollowed'}, status=status.HTTP_200_OK)


class HashtagViewSet(viewsets.ReadOnlyModelViewSet):
    queryset = Hashtag.objects.all()
    serializer_class = HashtagSerializer
    permission_classes = [AllowAny]
    filter_backends = [filters.SearchFilter]
    search_fields = ['name']
