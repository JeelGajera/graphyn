// UserService is declared in another file. Tier 2 does not resolve across
// files, so this reference records no cross-file edge.
namespace Example
{
    public class Elsewhere
    {
        public void Run()
        {
            var service = new UserService();
            service.Handle();
        }
    }
}
