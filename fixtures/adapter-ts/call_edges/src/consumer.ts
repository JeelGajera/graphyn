import { UserService, formatName as fmt } from "./services";

export function run(): string {
  // Instantiation of an imported class.
  const service = new UserService();

  // Call through a renamed import — resolves to the canonical symbol.
  const name = fmt("Ada", "Lovelace");

  // Member call: recorded as a property access on the receiver's type,
  // never as a call edge to the type itself.
  service.handle();

  // Neither of these names a symbol this file can resolve, so no edge.
  setTimeout(() => {}, 0);
  console.log(name);

  return name;
}
