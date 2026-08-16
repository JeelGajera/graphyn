#include "../include/user_payload.h"
#include <stdio.h>

typedef struct UserPayload ResponseModel;

void describe(ResponseModel *payload, Order *order) {
    printf("%s", payload->email);
    printf("%s", payload->user_id);
    printf("%s", order->order_id);
}

struct UserPayload *find(void);
