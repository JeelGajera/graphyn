# Everything below is resolvable inside this one file, which is exactly the
# limit of Tier 2: the analyzer records what it can see here and nothing more.
module Auditing
  def describe
    "user service"
  end
end

class AuditLog
  def record(message)
    message
  end
end

class UserService
  include Auditing

  def initialize
    @log = AuditLog.new
  end

  def handle
    # A call to a method defined in this file.
    describe
  end
end
