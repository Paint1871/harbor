import { Button } from "@harbor/ui/Button";
import { Card } from "@harbor/ui/Card";

export function ApprovalCard() {
  return (
    <Card>
      <p>Plugin writes need approval. Connection is not a grant.</p>
      <Button>Allow</Button>
      <Button variant="ghost">Deny</Button>
    </Card>
  );
}
