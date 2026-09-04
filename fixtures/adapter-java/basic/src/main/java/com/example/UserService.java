package com.example;

// Everything below is resolvable inside this one file, which is exactly the
// limit of Tier 2: the analyzer records what it can see here and nothing more.
public interface Auditable {
    String describe();
}

class AuditLog {
    void record(String message) {
    }
}

public class UserService implements Auditable {
    private AuditLog log;

    public String describe() {
        return "user service";
    }

    public void handle() {
        // A call to a method defined in this file.
        describe();
    }
}
