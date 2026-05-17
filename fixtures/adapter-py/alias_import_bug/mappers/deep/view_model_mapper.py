from ...models.user_payload import UserPayload as ResponseModel

class ViewModelMapper:
    def map(self, data: ResponseModel) -> dict:
        return {
            "id": data.user_id,
            "ts": data.timestamp,
            "st": data.status,
        }
