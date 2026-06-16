from django.core.management.base import BaseCommand
from django.contrib.auth.models import User
from core.models import Tweet, Like, Follow, Hashtag, TweetHashtag
from datetime import datetime, timedelta
import random


class Command(BaseCommand):
    help = 'Seed database with realistic sports data'

    def handle(self, *args, **options):
        self.stdout.write('🏆 Creating PlayThread sports athletes...')

        # Clear existing data
        Tweet.objects.all().delete()
        User.objects.filter(username__in=['messi', 'ronaldo', 'mbappe', 'haaland', 'neymar', 'benzema']).delete()

        # Create realistic sports athletes
        athletes_data = [
            {'username': 'messi', 'first_name': 'Lionel', 'email': 'messi@playthread.com'},
            {'username': 'ronaldo', 'first_name': 'Cristiano', 'email': 'ronaldo@playthread.com'},
            {'username': 'mbappe', 'first_name': 'Kylian', 'email': 'mbappe@playthread.com'},
            {'username': 'haaland', 'first_name': 'Erling', 'email': 'haaland@playthread.com'},
            {'username': 'neymar', 'first_name': 'Neymar', 'email': 'neymar@playthread.com'},
            {'username': 'benzema', 'first_name': 'Karim', 'email': 'benzema@playthread.com'},
        ]

        athletes = {}
        for data in athletes_data:
            user, _ = User.objects.get_or_create(
                username=data['username'],
                defaults={'first_name': data['first_name'], 'email': data['email']}
            )
            athletes[data['username']] = user
            self.stdout.write(f"✓ Created athlete: {data['first_name']}")

        # Create hashtags
        hashtags_data = ['Football', 'Champions League', 'World Cup', 'Goals', 'Assists', 'Soccer']
        hashtags = {}
        for tag_name in hashtags_data:
            tag, _ = Hashtag.objects.get_or_create(name=tag_name)
            hashtags[tag_name] = tag

        # Create realistic sports tweets
        sports_tweets = [
            {
                'author': athletes['messi'],
                'content': 'What an incredible match! The team played with passion and determination. #Football #Champions League',
                'hashtags': ['Football', 'Champions League']
            },
            {
                'author': athletes['ronaldo'],
                'content': 'Another goal for the record books! Grateful for every opportunity to play this beautiful game. 🎯 #Goals',
                'hashtags': ['Goals', 'Football']
            },
            {
                'author': athletes['mbappe'],
                'content': 'Training hard for the upcoming matches. Champions are made in the gym! 💪 #Football',
                'hashtags': ['Football']
            },
            {
                'author': athletes['haaland'],
                'content': 'Excited for the weekend match. Time to show what we are capable of! ⚽ #Football',
                'hashtags': ['Football']
            },
            {
                'author': athletes['neymar'],
                'content': 'Beautiful day for football! Thanks to the fans for the endless support 🙏 #Soccer',
                'hashtags': ['Soccer', 'Football']
            },
            {
                'author': athletes['benzema'],
                'content': 'Experience is the best teacher. Every match makes us stronger. #Champions League #Football',
                'hashtags': ['Champions League', 'Football']
            },
            {
                'author': athletes['messi'],
                'content': 'Winning together is what makes this sport special. Proud of my teammates! 👏 #Football',
                'hashtags': ['Football']
            },
            {
                'author': athletes['ronaldo'],
                'content': 'Consistency is key to success. Keep pushing, keep improving! 💯 #Goals #Football',
                'hashtags': ['Goals', 'Football']
            },
            {
                'author': athletes['mbappe'],
                'content': 'Speed and precision - the perfect combination! Working on my game every single day. #Football',
                'hashtags': ['Football']
            },
            {
                'author': athletes['haaland'],
                'content': 'The beautiful game brings joy to millions. Honored to be part of this journey ⚽ #Soccer #Football',
                'hashtags': ['Soccer', 'Football']
            },
        ]

        # Create tweets with varying dates
        base_date = datetime.now()
        for i, tweet_data in enumerate(sports_tweets):
            created_at = base_date - timedelta(days=i, hours=random.randint(0, 23))
            tweet = Tweet.objects.create(
                author=tweet_data['author'],
                content=tweet_data['content'],
                created_at=created_at
            )

            # Add hashtags to tweet
            for hashtag_name in tweet_data['hashtags']:
                hashtag = hashtags[hashtag_name]
                TweetHashtag.objects.create(tweet=tweet, hashtag=hashtag)

            self.stdout.write(f"✓ Created tweet from {tweet_data['author'].first_name}")

        # Create some likes between athletes
        tweets = Tweet.objects.all()
        for tweet in tweets[:5]:
            for athlete in list(athletes.values())[:3]:
                if athlete != tweet.author:
                    Like.objects.get_or_create(user=athlete, tweet=tweet)

        # Create some follows
        athlete_list = list(athletes.values())
        for athlete in athlete_list:
            for other in athlete_list[1:3]:
                if athlete != other:
                    Follow.objects.get_or_create(follower=athlete, following=other)

        self.stdout.write(self.style.SUCCESS('✨ PlayThread database seeded with sports data!'))
