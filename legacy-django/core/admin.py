from django.contrib import admin
from django.contrib.auth.models import User
from .models import Profile, Tweet, Like, Retweet, Follow, Hashtag, TweetHashtag


@admin.register(Profile)
class ProfileAdmin(admin.ModelAdmin):
    list_display = ['user', 'followers_count', 'following_count', 'tweets_count', 'is_verified']
    list_filter = ['is_verified', 'created_at']
    search_fields = ['user__username', 'bio']
    readonly_fields = ['followers_count', 'following_count', 'tweets_count', 'created_at']


class ReplyInline(admin.TabularInline):
    model = Tweet
    extra = 0
    fields = ['author', 'content', 'created_at']
    readonly_fields = ['created_at']
    fk_name = 'parent_tweet'
    can_delete = True


class LikeInline(admin.TabularInline):
    model = Like
    extra = 0
    fields = ['user', 'created_at']
    readonly_fields = ['created_at']
    can_delete = True


class RetweetInline(admin.TabularInline):
    model = Retweet
    extra = 0
    fields = ['user', 'created_at']
    readonly_fields = ['created_at']
    can_delete = True


@admin.register(Tweet)
class TweetAdmin(admin.ModelAdmin):
    list_display = ['author', 'content_preview', 'likes_count', 'retweets_count', 'replies_count', 'created_at']
    list_filter = ['created_at']
    search_fields = ['content', 'author__username']
    readonly_fields = ['likes_count', 'retweets_count', 'replies_count', 'created_at', 'updated_at']
    inlines = [ReplyInline, LikeInline, RetweetInline]

    def content_preview(self, obj):
        return obj.content[:50] + '...' if len(obj.content) > 50 else obj.content
    content_preview.short_description = 'Content'

    fieldsets = (
        ('Content', {'fields': ('content', 'author')}),
        ('Parent', {'fields': ('parent_tweet',), 'classes': ('collapse',)}),
        ('Stats', {'fields': ('likes_count', 'retweets_count', 'replies_count')}),
        ('Timestamps', {'fields': ('created_at', 'updated_at')}),
    )


@admin.register(Like)
class LikeAdmin(admin.ModelAdmin):
    list_display = ['user', 'tweet_preview', 'created_at']
    list_filter = ['created_at']
    search_fields = ['user__username', 'tweet__content']
    readonly_fields = ['created_at']

    def tweet_preview(self, obj):
        return obj.tweet.content[:30] + '...'
    tweet_preview.short_description = 'Tweet'


@admin.register(Retweet)
class RetweetAdmin(admin.ModelAdmin):
    list_display = ['user', 'tweet_preview', 'created_at']
    list_filter = ['created_at']
    search_fields = ['user__username', 'tweet__content']
    readonly_fields = ['created_at']

    def tweet_preview(self, obj):
        return obj.tweet.content[:30] + '...'
    tweet_preview.short_description = 'Tweet'


@admin.register(Follow)
class FollowAdmin(admin.ModelAdmin):
    list_display = ['follower', 'following', 'created_at']
    list_filter = ['created_at']
    search_fields = ['follower__username', 'following__username']
    readonly_fields = ['created_at']


@admin.register(Hashtag)
class HashtagAdmin(admin.ModelAdmin):
    list_display = ['name', 'slug', 'tweets_count', 'created_at']
    list_filter = ['created_at']
    search_fields = ['name']
    readonly_fields = ['slug', 'tweets_count', 'created_at']


@admin.register(TweetHashtag)
class TweetHashtagAdmin(admin.ModelAdmin):
    list_display = ['tweet_preview', 'hashtag', 'created_at']
    list_filter = ['created_at']
    search_fields = ['tweet__content', 'hashtag__name']
    readonly_fields = ['created_at']

    def tweet_preview(self, obj):
        return obj.tweet.content[:30] + '...'
    tweet_preview.short_description = 'Tweet'
