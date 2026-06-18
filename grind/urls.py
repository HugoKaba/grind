from django.contrib import admin
from django.urls import path, include
from django.contrib.auth import views as auth_views
from drf_spectacular.views import SpectacularAPIView, SpectacularSwaggerView, SpectacularRedocView

urlpatterns = [
    path('admin/', admin.site.urls),

    # Swagger/OpenAPI
    path('api/schema/', SpectacularAPIView.as_view(), name='schema'),
    path('api/swagger/', SpectacularSwaggerView.as_view(url_name='schema'), name='swagger-ui'),
    path('api/redoc/', SpectacularRedocView.as_view(url_name='schema'), name='redoc'),

    # API
    path('api/', include('api.urls')),
    path('api-auth/', include('rest_framework.urls')),
    path('', include('core.urls')),

    # Auth
    path('accounts/login/', auth_views.LoginView.as_view(template_name='registration/login.html'), name='login'),
]
