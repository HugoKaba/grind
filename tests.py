from django.test import TestCase, Client
from django.contrib.auth.models import User
from rest_framework.test import APIClient, APITestCase
from rest_framework import status

from core.models import Question, Answer, Tag, Comment, Vote


class TagModelTest(TestCase):
    def setUp(self):
        self.tag = Tag.objects.create(name='Python', description='Python programming')

    def test_tag_creation(self):
        self.assertEqual(self.tag.name, 'Python')
        self.assertEqual(str(self.tag), 'Python')


class QuestionModelTest(TestCase):
    def setUp(self):
        self.user = User.objects.create_user(username='testuser', password='test123')
        self.tag = Tag.objects.create(name='Django')
        self.question = Question.objects.create(
            title='How to use Django?', content='I want to learn Django', author=self.user
        )
        self.question.tags.add(self.tag)

    def test_question_creation(self):
        self.assertEqual(self.question.title, 'How to use Django?')
        self.assertEqual(self.question.author, self.user)
        self.assertIn(self.tag, self.question.tags.all())

    def test_question_answer_count(self):
        self.assertEqual(self.question.answer_count, 0)
        Answer.objects.create(question=self.question, author=self.user, content='Answer text')
        self.assertEqual(self.question.answer_count, 1)


class AnswerModelTest(TestCase):
    def setUp(self):
        self.user = User.objects.create_user(username='testuser', password='test123')
        self.question = Question.objects.create(
            title='Test Question', content='Test content', author=self.user
        )
        self.answer = Answer.objects.create(
            question=self.question, author=self.user, content='Test answer'
        )

    def test_answer_creation(self):
        self.assertEqual(self.answer.question, self.question)
        self.assertEqual(self.answer.author, self.user)

    def test_answer_acceptance(self):
        self.assertFalse(self.answer.is_accepted)
        self.answer.is_accepted = True
        self.answer.save()
        self.assertTrue(Answer.objects.get(pk=self.answer.pk).is_accepted)


class QuestionAPITest(APITestCase):
    def setUp(self):
        self.client = APIClient()
        self.user = User.objects.create_user(username='testuser', password='test123')
        self.tag = Tag.objects.create(name='Django')
        self.question = Question.objects.create(
            title='Test Question', content='Test content', author=self.user
        )
        self.question.tags.add(self.tag)

    def test_get_questions_list(self):
        response = self.client.get('/api/questions/')
        self.assertEqual(response.status_code, status.HTTP_200_OK)
        self.assertEqual(response.data['count'], 1)

    def test_get_question_detail(self):
        response = self.client.get(f'/api/questions/{self.question.id}/')
        self.assertEqual(response.status_code, status.HTTP_200_OK)
        self.assertEqual(response.data['title'], 'Test Question')

    def test_create_question_authenticated(self):
        self.client.force_authenticate(user=self.user)
        data = {'title': 'New Question', 'content': 'New content', 'tags': [self.tag.id]}
        response = self.client.post('/api/questions/', data, format='json')
        self.assertEqual(response.status_code, status.HTTP_201_CREATED)
        self.assertEqual(Question.objects.count(), 2)

    def test_create_question_unauthenticated(self):
        data = {'title': 'New Question', 'content': 'New content', 'tags': []}
        response = self.client.post('/api/questions/', data, format='json')
        self.assertEqual(response.status_code, status.HTTP_401_UNAUTHORIZED)

    def test_update_own_question(self):
        self.client.force_authenticate(user=self.user)
        data = {'title': 'Updated Title', 'content': 'Updated content', 'tags': [self.tag.id]}
        response = self.client.put(
            f'/api/questions/{self.question.id}/', data, format='json'
        )
        self.assertEqual(response.status_code, status.HTTP_200_OK)
        self.question.refresh_from_db()
        self.assertEqual(self.question.title, 'Updated Title')

    def test_update_others_question(self):
        other_user = User.objects.create_user(username='otheruser', password='test123')
        self.client.force_authenticate(user=other_user)
        data = {'title': 'Hacked Title', 'content': 'Hacked content', 'tags': []}
        response = self.client.put(
            f'/api/questions/{self.question.id}/', data, format='json'
        )
        self.assertEqual(response.status_code, status.HTTP_403_FORBIDDEN)

    def test_delete_own_question(self):
        self.client.force_authenticate(user=self.user)
        response = self.client.delete(f'/api/questions/{self.question.id}/')
        self.assertEqual(response.status_code, status.HTTP_204_NO_CONTENT)
        self.assertEqual(Question.objects.count(), 0)

    def test_question_views_increment(self):
        initial_views = self.question.views
        response = self.client.get(f'/api/questions/{self.question.id}/')
        self.question.refresh_from_db()
        self.assertEqual(self.question.views, initial_views + 1)

    def test_upvote_question(self):
        self.client.force_authenticate(user=self.user)
        response = self.client.post(f'/api/questions/{self.question.id}/upvote/')
        self.assertEqual(response.status_code, status.HTTP_200_OK)
        self.assertEqual(self.question.votes_count, 1)


class AnswerAPITest(APITestCase):
    def setUp(self):
        self.client = APIClient()
        self.user = User.objects.create_user(username='testuser', password='test123')
        self.question = Question.objects.create(
            title='Test Question', content='Test content', author=self.user
        )
        self.answer = Answer.objects.create(
            question=self.question, author=self.user, content='Test answer'
        )

    def test_get_answers_list(self):
        response = self.client.get('/api/answers/')
        self.assertEqual(response.status_code, status.HTTP_200_OK)
        self.assertEqual(response.data['count'], 1)

    def test_create_answer(self):
        self.client.force_authenticate(user=self.user)
        data = {'question': self.question.id, 'content': 'New answer'}
        response = self.client.post('/api/answers/', data, format='json')
        self.assertEqual(response.status_code, status.HTTP_201_CREATED)
        self.assertEqual(Answer.objects.count(), 2)

    def test_accept_answer_by_question_author(self):
        self.client.force_authenticate(user=self.user)
        response = self.client.post(f'/api/answers/{self.answer.id}/accept/')
        self.assertEqual(response.status_code, status.HTTP_200_OK)
        self.answer.refresh_from_db()
        self.assertTrue(self.answer.is_accepted)

    def test_accept_answer_by_non_author(self):
        other_user = User.objects.create_user(username='otheruser', password='test123')
        self.client.force_authenticate(user=other_user)
        response = self.client.post(f'/api/answers/{self.answer.id}/accept/')
        self.assertEqual(response.status_code, status.HTTP_403_FORBIDDEN)


class VoteAPITest(APITestCase):
    def setUp(self):
        self.client = APIClient()
        self.user = User.objects.create_user(username='testuser', password='test123')
        self.question = Question.objects.create(
            title='Test Question', content='Test content', author=self.user
        )

    def test_upvote_question(self):
        self.client.force_authenticate(user=self.user)
        response = self.client.post(f'/api/questions/{self.question.id}/upvote/')
        self.assertEqual(response.status_code, status.HTTP_200_OK)
        self.assertTrue(Vote.objects.filter(author=self.user, question=self.question).exists())

    def test_change_vote(self):
        self.client.force_authenticate(user=self.user)
        self.client.post(f'/api/questions/{self.question.id}/upvote/')
        self.client.post(f'/api/questions/{self.question.id}/downvote/')
        vote = Vote.objects.get(author=self.user, question=self.question)
        self.assertEqual(vote.value, -1)

    def test_unvote(self):
        self.client.force_authenticate(user=self.user)
        self.client.post(f'/api/questions/{self.question.id}/upvote/')
        response = self.client.post(f'/api/questions/{self.question.id}/unvote/')
        self.assertEqual(response.status_code, status.HTTP_200_OK)
        self.assertFalse(Vote.objects.filter(author=self.user, question=self.question).exists())


class AuthenticationTest(APITestCase):
    def setUp(self):
        self.client = APIClient()
        self.user = User.objects.create_user(username='testuser', password='test123')

    def test_get_token(self):
        response = self.client.post(
            '/api/auth/token/', {'username': 'testuser', 'password': 'test123'}, format='json'
        )
        self.assertEqual(response.status_code, status.HTTP_200_OK)
        self.assertIn('access', response.data)

    def test_invalid_credentials(self):
        response = self.client.post(
            '/api/auth/token/', {'username': 'testuser', 'password': 'wrong'}, format='json'
        )
        self.assertEqual(response.status_code, status.HTTP_401_UNAUTHORIZED)


class WebInterfaceTest(TestCase):
    def setUp(self):
        self.client = Client()
        self.user = User.objects.create_user(username='testuser', password='test123')
        self.question = Question.objects.create(
            title='Test Question', content='Test content', author=self.user
        )

    def test_question_list_view(self):
        response = self.client.get('/')
        self.assertEqual(response.status_code, 200)
        self.assertContains(response, 'Questions')
        self.assertContains(response, 'Test Question')

    def test_question_detail_view(self):
        response = self.client.get(f'/questions/{self.question.id}/')
        self.assertEqual(response.status_code, 200)
        self.assertContains(response, 'Test Question')
        self.assertContains(response, 'testuser')

    def test_login_required_for_question_creation(self):
        response = self.client.get('/questions/new/')
        self.assertEqual(response.status_code, 302)
        self.assertTrue(response.url.startswith('/accounts/login/'))

    def test_create_question_authenticated(self):
        self.client.login(username='testuser', password='test123')
        response = self.client.get('/questions/new/')
        self.assertEqual(response.status_code, 200)
        self.assertContains(response, 'Ask a Question')
