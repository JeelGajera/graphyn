package com.example;

// UserService is declared in another file. Tier 2 does not resolve across
// files, so this reference records no edge — and `status` reports the tier so
// nobody mistakes that silence for "nothing uses UserService".
public class Elsewhere {
    public void run() {
        UserService service = new UserService();
        service.handle();
    }
}
