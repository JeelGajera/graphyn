// Everything below is resolvable inside this one file, which is exactly the
// limit of Tier 2: the analyzer records what it can see here and nothing more.
namespace Example
{
    public interface IAuditable
    {
        string Describe();
    }

    class AuditLog
    {
        public void Record(string message) { }
    }

    public class UserService : IAuditable
    {
        private AuditLog log;

        public string Describe()
        {
            return "user service";
        }

        public void Handle()
        {
            // A call to a method defined in this file.
            Describe();
        }
    }
}
