export interface UserData {
  name: string;
}

export const UserStore = {
  get: (): UserData => ({ name: "test" }),
  load: () => {},
};
