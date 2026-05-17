#include "../include/user_payload.hpp"

using ResponseModel = UserPayload;

void map(ResponseModel *data) {
    auto a = data->user_id;
    auto b = data->timestamp;
    auto c = data->status;
}
