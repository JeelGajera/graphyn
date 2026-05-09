import { UserRepository } from './user.repository';
import { EmailService } from './email.service';

export class PaymentService {
  constructor(
    private readonly userRepo: UserRepository,
    private readonly emailService: EmailService,
  ) {}

  async processPayment(userId: string): Promise<void> {
    const user = await this.userRepo.findById(userId);
    await this.emailService.sendReceipt('test@test.com');
  }
}
