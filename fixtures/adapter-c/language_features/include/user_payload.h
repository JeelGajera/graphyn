#ifndef USER_PAYLOAD_H
#define USER_PAYLOAD_H

typedef struct UserPayload {
    char *user_id;
    char *email;
    char *timestamp;
} UserPayload;

typedef struct Order {
    char *order_id;
    long total;
} Order;

#endif
