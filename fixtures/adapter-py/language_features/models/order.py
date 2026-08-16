from django.db import models


class Order(models.Model):
    order_id = models.CharField(max_length=64)
    total = models.IntegerField()
