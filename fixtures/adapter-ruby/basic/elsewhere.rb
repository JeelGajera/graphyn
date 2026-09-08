# UserService is declared in another file. Tier 2 does not resolve across
# files, so this reference records no cross-file edge — and `status` reports
# the tier so nobody mistakes that silence for "nothing uses UserService".
require_relative "user_service"

class Elsewhere
  def run
    service = UserService.new
    service.handle
  end
end
