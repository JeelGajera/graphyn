#include "../include/user_payload.h"

typedef struct UserPayload ResponseModel;

void map(ResponseModel *data) {
    data->user_id = data->user_id;
    data->timestamp = data->timestamp;
    data->status = data->status;
}
