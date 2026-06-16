from rest_framework import serializers
from django.contrib.auth.models import User
from core.models import Tweet, Like, Retweet, Follow, Hashtag, TweetHashtag, Profile


class UserSerializer(serializers.ModelSerializer):
    class Meta:
        model = User
        fields = ['id', 'username', 'first_name', 'last_name']
        read_only_fields = ['id']


class ProfileSerializer(serializers.ModelSerializer):
    user = UserSerializer(read_only=True)

    class Meta:
        model = Profile
        fields = [
            'id', 'user', 'bio', 'location', 'website', 'avatar', 'cover',
            'is_verified', 'followers_count', 'following_count', 'tweets_count', 'created_at'
        ]
        read_only_fields = ['id', 'followers_count', 'following_count', 'tweets_count', 'created_at']


class HashtagSerializer(serializers.ModelSerializer):
    class Meta:
        model = Hashtag
        fields = ['id', 'name', 'slug', 'tweets_count', 'created_at']
        read_only_fields = ['id', 'slug', 'tweets_count', 'created_at']


class LikeSerializer(serializers.ModelSerializer):
    user = UserSerializer(read_only=True)

    class Meta:
        model = Like
        fields = ['id', 'user', 'tweet', 'created_at']
        read_only_fields = ['id', 'user', 'created_at']


class RetweetSerializer(serializers.ModelSerializer):
    user = UserSerializer(read_only=True)

    class Meta:
        model = Retweet
        fields = ['id', 'user', 'tweet', 'created_at']
        read_only_fields = ['id', 'user', 'created_at']


class FollowSerializer(serializers.ModelSerializer):
    follower = UserSerializer(read_only=True)
    following = UserSerializer(read_only=True)

    class Meta:
        model = Follow
        fields = ['id', 'follower', 'following', 'created_at']
        read_only_fields = ['id', 'created_at']


class TweetListSerializer(serializers.ModelSerializer):
    author = UserSerializer(read_only=True)
    is_liked = serializers.SerializerMethodField()
    is_retweeted = serializers.SerializerMethodField()

    class Meta:
        model = Tweet
        fields = [
            'id', 'author', 'content', 'parent_tweet', 'likes_count',
            'retweets_count', 'replies_count', 'is_liked', 'is_retweeted', 'created_at'
        ]
        read_only_fields = ['id', 'author', 'likes_count', 'retweets_count', 'replies_count', 'created_at']

    def get_is_liked(self, obj):
        request = self.context.get('request')
        if request and request.user.is_authenticated:
            return Like.objects.filter(user=request.user, tweet=obj).exists()
        return False

    def get_is_retweeted(self, obj):
        request = self.context.get('request')
        if request and request.user.is_authenticated:
            return Retweet.objects.filter(user=request.user, tweet=obj).exists()
        return False


class TweetDetailSerializer(serializers.ModelSerializer):
    author = ProfileSerializer(read_only=True)
    replies = TweetListSerializer(many=True, read_only=True)
    liked_by = LikeSerializer(many=True, read_only=True)
    retweeted_by = RetweetSerializer(many=True, read_only=True)
    hashtags = HashtagSerializer(many=True, read_only=True)
    is_liked = serializers.SerializerMethodField()
    is_retweeted = serializers.SerializerMethodField()

    class Meta:
        model = Tweet
        fields = [
            'id', 'author', 'content', 'parent_tweet', 'replies', 'liked_by',
            'retweeted_by', 'hashtags', 'likes_count', 'retweets_count', 'replies_count',
            'is_liked', 'is_retweeted', 'created_at', 'updated_at'
        ]
        read_only_fields = ['id', 'author', 'replies', 'liked_by', 'retweeted_by', 'hashtags',
                          'likes_count', 'retweets_count', 'replies_count', 'created_at', 'updated_at']

    def get_is_liked(self, obj):
        request = self.context.get('request')
        if request and request.user.is_authenticated:
            return Like.objects.filter(user=request.user, tweet=obj).exists()
        return False

    def get_is_retweeted(self, obj):
        request = self.context.get('request')
        if request and request.user.is_authenticated:
            return Retweet.objects.filter(user=request.user, tweet=obj).exists()
        return False


class TweetCreateSerializer(serializers.ModelSerializer):
    class Meta:
        model = Tweet
        fields = ['content']

    def create(self, validated_data):
        tweet = Tweet.objects.create(**validated_data)
        # Extract hashtags from content
        import re
        hashtags = re.findall(r'#(\w+)', validated_data['content'])
        for hashtag_text in hashtags:
            hashtag, _ = Hashtag.objects.get_or_create(name=hashtag_text)
            TweetHashtag.objects.create(tweet=tweet, hashtag=hashtag)
            hashtag.tweets_count += 1
            hashtag.save(update_fields=['tweets_count'])
        return tweet
