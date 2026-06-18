from django.shortcuts import render, redirect, get_object_or_404
from django.contrib.auth.decorators import login_required
from django.contrib.auth import logout
from django.views.generic import ListView, DetailView
from django.contrib.auth.mixins import LoginRequiredMixin
from django.db.models import Q
from django.contrib.auth.models import User

from .models import Tweet, Like, Retweet, Follow, Hashtag, Profile, Bookmark, Message, Notification


class TimelineView(ListView):
    model = Tweet
    template_name = 'core/timeline.html'
    context_object_name = 'tweets'
    paginate_by = 20

    def get_queryset(self):
        if self.request.user.is_authenticated:
            # Get tweets from people user follows + own tweets
            following_users = Follow.objects.filter(follower=self.request.user).values_list('following_id')
            return Tweet.objects.filter(
                Q(author_id__in=following_users) | Q(author=self.request.user)
            ).select_related('author').prefetch_related('liked_by', 'retweeted_by', 'replies')
        else:
            # Show trending tweets for non-authenticated users
            return Tweet.objects.filter(parent_tweet__isnull=True).select_related('author').prefetch_related('liked_by', 'retweeted_by', 'replies')

    def get_context_data(self, **kwargs):
        context = super().get_context_data(**kwargs)
        if self.request.user.is_authenticated:
            # Get all tweets (already in self.tweets)
            context['trending_hashtags'] = Hashtag.objects.all().order_by('-tweets_count')[:10]

            # Get only friend tweets
            following_users = Follow.objects.filter(follower=self.request.user).values_list('following_id')
            context['friend_tweets'] = Tweet.objects.filter(
                Q(author_id__in=following_users) | Q(author=self.request.user),
                parent_tweet__isnull=True
            ).select_related('author').order_by('-created_at')[:20]
        return context


class ProfileView(DetailView):
    model = User
    template_name = 'core/profile.html'
    context_object_name = 'profile_user'
    slug_field = 'username'
    slug_url_kwarg = 'username'

    def get_queryset(self):
        return User.objects.select_related('profile').prefetch_related('tweets', 'profile__user__followers', 'profile__user__following')

    def get_context_data(self, **kwargs):
        context = super().get_context_data(**kwargs)
        user = self.get_object()
        context['tweets'] = user.tweets.filter(parent_tweet__isnull=True).order_by('-created_at')
        context['followers'] = user.followers.count()
        context['following'] = user.following.count()
        if self.request.user.is_authenticated:
            context['is_following'] = Follow.objects.filter(
                follower=self.request.user, following=user
            ).exists()
        return context


@login_required
def post_tweet(request):
    if request.method == 'POST':
        content = request.POST.get('content')
        if content and len(content) <= 280:
            tweet = Tweet.objects.create(author=request.user, content=content)
            # Update profile tweets count
            request.user.profile.tweets_count += 1
            request.user.profile.save(update_fields=['tweets_count'])
            return redirect('timeline')

    return render(request, 'core/post_tweet.html')


@login_required
def like_tweet(request, tweet_id):
    tweet = get_object_or_404(Tweet, pk=tweet_id)
    like, created = Like.objects.get_or_create(user=request.user, tweet=tweet)
    if created:
        tweet.likes_count += 1
        tweet.save(update_fields=['likes_count'])
    return redirect(request.META.get('HTTP_REFERER', 'timeline'))


@login_required
def unlike_tweet(request, tweet_id):
    tweet = get_object_or_404(Tweet, pk=tweet_id)
    deleted, _ = Like.objects.filter(user=request.user, tweet=tweet).delete()
    if deleted:
        tweet.likes_count = max(0, tweet.likes_count - 1)
        tweet.save(update_fields=['likes_count'])
    return redirect(request.META.get('HTTP_REFERER', 'timeline'))


@login_required
def retweet(request, tweet_id):
    tweet = get_object_or_404(Tweet, pk=tweet_id)
    retweet, created = Retweet.objects.get_or_create(user=request.user, tweet=tweet)
    if created:
        tweet.retweets_count += 1
        tweet.save(update_fields=['retweets_count'])
    return redirect(request.META.get('HTTP_REFERER', 'timeline'))


@login_required
def unretweet(request, tweet_id):
    tweet = get_object_or_404(Tweet, pk=tweet_id)
    deleted, _ = Retweet.objects.filter(user=request.user, tweet=tweet).delete()
    if deleted:
        tweet.retweets_count = max(0, tweet.retweets_count - 1)
        tweet.save(update_fields=['retweets_count'])
    return redirect(request.META.get('HTTP_REFERER', 'timeline'))


@login_required
def reply_tweet(request, tweet_id):
    parent_tweet = get_object_or_404(Tweet, pk=tweet_id)
    if request.method == 'POST':
        content = request.POST.get('content')
        if content and len(content) <= 280:
            reply = Tweet.objects.create(author=request.user, content=content, parent_tweet=parent_tweet)
            parent_tweet.replies_count += 1
            parent_tweet.save(update_fields=['replies_count'])
            return redirect('tweet-detail', pk=parent_tweet.pk)

    context = {'parent_tweet': parent_tweet}
    return render(request, 'core/reply_tweet.html', context)


class TweetDetailView(DetailView):
    model = Tweet
    template_name = 'core/tweet_detail.html'
    context_object_name = 'tweet'

    def get_context_data(self, **kwargs):
        context = super().get_context_data(**kwargs)
        tweet = self.get_object()
        context['replies'] = tweet.replies.all().order_by('-created_at')
        return context


@login_required
def follow_user(request, username):
    user_to_follow = get_object_or_404(User, username=username)
    if request.user != user_to_follow:
        follow, created = Follow.objects.get_or_create(follower=request.user, following=user_to_follow)
        if created:
            request.user.profile.following_count += 1
            user_to_follow.profile.followers_count += 1
            request.user.profile.save(update_fields=['following_count'])
            user_to_follow.profile.save(update_fields=['followers_count'])
    return redirect('profile', username=username)


@login_required
def unfollow_user(request, username):
    user_to_unfollow = get_object_or_404(User, username=username)
    deleted, _ = Follow.objects.filter(follower=request.user, following=user_to_unfollow).delete()
    if deleted:
        request.user.profile.following_count = max(0, request.user.profile.following_count - 1)
        user_to_unfollow.profile.followers_count = max(0, user_to_unfollow.profile.followers_count - 1)
        request.user.profile.save(update_fields=['following_count'])
        user_to_unfollow.profile.save(update_fields=['followers_count'])
    return redirect('profile', username=username)


@login_required
def notifications(request):
    notifs = Notification.objects.filter(user=request.user).select_related('actor', 'tweet').order_by('-created_at')[:50]
    unread_count = Notification.objects.filter(user=request.user, is_read=False).count()
    for notif in notifs:
        if not notif.is_read:
            notif.is_read = True
            notif.save(update_fields=['is_read'])
    context = {'notifications': notifs, 'unread_count': unread_count}
    return render(request, 'core/notifications.html', context)


@login_required
def trending(request):
    trending_hashtags = Hashtag.objects.all().order_by('-tweets_count')[:20]
    context = {'trending_hashtags': trending_hashtags}
    return render(request, 'core/trending.html', context)


@login_required
def bookmarks(request):
    bookmarks_list = Bookmark.objects.filter(user=request.user).select_related('tweet', 'tweet__author').order_by('-created_at')
    tweets = [b.tweet for b in bookmarks_list]
    context = {'tweets': tweets, 'bookmarks_count': len(tweets)}
    return render(request, 'core/bookmarks.html', context)


@login_required
def bookmark_tweet(request, tweet_id):
    tweet = get_object_or_404(Tweet, pk=tweet_id)
    bookmark, created = Bookmark.objects.get_or_create(user=request.user, tweet=tweet)
    return redirect(request.META.get('HTTP_REFERER', 'timeline'))


@login_required
def unbookmark_tweet(request, tweet_id):
    tweet = get_object_or_404(Tweet, pk=tweet_id)
    Bookmark.objects.filter(user=request.user, tweet=tweet).delete()
    return redirect(request.META.get('HTTP_REFERER', 'timeline'))


@login_required
def messages(request):
    # Only show conversations with people user follows
    following_users = Follow.objects.filter(follower=request.user).values_list('following_id', flat=True)

    conversations = []
    received = Message.objects.filter(recipient=request.user, sender_id__in=following_users).values_list('sender_id', flat=True).distinct()
    sent = Message.objects.filter(sender=request.user, recipient_id__in=following_users).values_list('recipient_id', flat=True).distinct()

    user_ids = set(received) | set(sent)
    for user_id in user_ids:
        user = User.objects.get(pk=user_id)
        last_msg = Message.objects.filter(
            Q(sender=request.user, recipient=user) | Q(sender=user, recipient=request.user)
        ).order_by('-created_at').first()
        unread = Message.objects.filter(sender=user, recipient=request.user, is_read=False).count()
        conversations.append({'user': user, 'last_message': last_msg, 'unread': unread})

    context = {'conversations': sorted(conversations, key=lambda x: x['last_message'].created_at if x['last_message'] else None, reverse=True)}
    return render(request, 'core/messages.html', context)


@login_required
def message_thread(request, username):
    other_user = get_object_or_404(User, username=username)
    if request.method == 'POST':
        content = request.POST.get('content')
        if content:
            Message.objects.create(sender=request.user, recipient=other_user, content=content)
            return redirect('message-thread', username=username)

    messages_list = Message.objects.filter(
        Q(sender=request.user, recipient=other_user) | Q(sender=other_user, recipient=request.user)
    ).order_by('created_at')

    Message.objects.filter(sender=other_user, recipient=request.user).update(is_read=True)

    context = {'other_user': other_user, 'messages': messages_list}
    return render(request, 'core/message_thread.html', context)


@login_required
def new_message(request):
    # Only show people user follows for new messages
    following_users = Follow.objects.filter(follower=request.user).values_list('following_id', flat=True)
    users = User.objects.filter(id__in=following_users)
    context = {'users': users}
    return render(request, 'core/new_message.html', context)


def api_docs(request):
    return render(request, 'api_docs.html')


@login_required
def logout_view(request):
    logout(request)
    return redirect('login')
