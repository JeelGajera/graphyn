import { UserPayload as ResponseModel } from '../../models/user_payload';

export class ViewModelMapper {
  map(data: ResponseModel): object {
    return {
      id: data.userId,
      ts: data.timestamp,
      st: data.status,
    };
  }
}
