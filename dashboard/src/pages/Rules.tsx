import { useState } from "react";
import { useRules, useAddRule, useDeleteRule, useTestRule } from "@/hooks/use-api";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { ErrorAlert } from "@/components/ErrorAlert";
import { Skeleton } from "@/components/Skeleton";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "@/components/ui/table";
import { Trash2, Plus, FlaskConical, FileText } from "lucide-react";

export function Rules() {
  const { data: rules, isLoading, error, refetch } = useRules();
  const addRule = useAddRule();
  const deleteRule = useDeleteRule();
  const testRule = useTestRule();

  const [newRule, setNewRule] = useState("");
  const [testRuleInput, setTestRuleInput] = useState("");
  const [testMethod, setTestMethod] = useState("GET");
  const [testUri, setTestUri] = useState("/");
  const [testResult, setTestResult] = useState<string | null>(null);

  function handleAddRule() {
    if (!newRule.trim()) return;
    addRule.mutate(newRule, {
      onSuccess: () => setNewRule(""),
    });
  }

  function handleTestRule() {
    if (!testRuleInput.trim()) return;
    testRule.mutate(
      {
        rule: testRuleInput,
        request: {
          method: testMethod,
          uri: testUri,
          headers: {},
        },
      },
      {
        onSuccess: (data) => {
          setTestResult(
            data.matched
              ? `MATCHED: ${data.message}`
              : `NOT MATCHED: ${data.message}`
          );
        },
        onError: (err) => {
          setTestResult(`Error: ${err.message}`);
        },
      }
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">WAF Rules</h2>
        <p className="text-muted-foreground text-sm">
          Manage rule files and custom WAF rules
        </p>
      </div>

      {error && (
        <ErrorAlert message={error.message} onRetry={() => refetch()} />
      )}

      {/* Rule files */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <FileText className="h-4 w-4" />
            Rule Files
          </CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-2">
              <Skeleton className="h-6 w-48" />
              <Skeleton className="h-6 w-56" />
            </div>
          ) : rules && rules.rule_files.length > 0 ? (
            <div className="space-y-2">
              {rules.rule_files.map((file, i) => (
                <div key={i} className="flex items-center gap-2">
                  <Badge variant="outline" className="font-mono text-xs">
                    {file}
                  </Badge>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">
              No rule files configured
            </p>
          )}
        </CardContent>
      </Card>

      {/* Custom rules */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Custom Rules</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-16">ID</TableHead>
                <TableHead>Rule</TableHead>
                <TableHead className="w-20">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <TableRow>
                  <TableCell colSpan={3} className="text-center text-muted-foreground py-4">
                    Loading...
                  </TableCell>
                </TableRow>
              ) : rules && rules.custom_rules.length > 0 ? (
                rules.custom_rules.map((cr) => (
                  <TableRow key={cr.id}>
                    <TableCell className="font-mono">{cr.id}</TableCell>
                    <TableCell className="font-mono text-xs break-all">
                      {cr.rule}
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => deleteRule.mutate(cr.id)}
                        disabled={deleteRule.isPending}
                      >
                        <Trash2 className="h-4 w-4 text-destructive-foreground" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))
              ) : (
                <TableRow>
                  <TableCell colSpan={3} className="text-center text-muted-foreground py-4">
                    No custom rules
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>

          <Separator />

          {/* Add rule form */}
          <div className="space-y-2">
            <Label>Add Custom Rule</Label>
            <Textarea
              placeholder='SecRule ARGS "@contains test" "id:1000,phase:1,deny,status:403"'
              value={newRule}
              onChange={(e) => setNewRule(e.target.value)}
              rows={3}
              className="font-mono text-xs"
            />
            <Button onClick={handleAddRule} disabled={addRule.isPending || !newRule.trim()}>
              <Plus className="h-4 w-4 mr-1" />
              Add Rule
            </Button>
            {addRule.isError && (
              <p className="text-sm text-destructive-foreground">{addRule.error.message}</p>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Test rule */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <FlaskConical className="h-4 w-4" />
            Test Rule
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>Rule</Label>
            <Textarea
              placeholder='SecRule ARGS "@contains attack" "id:9999,phase:1,deny"'
              value={testRuleInput}
              onChange={(e) => setTestRuleInput(e.target.value)}
              rows={2}
              className="font-mono text-xs"
            />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>Method</Label>
              <Input
                value={testMethod}
                onChange={(e) => setTestMethod(e.target.value)}
                placeholder="GET"
              />
            </div>
            <div className="space-y-2">
              <Label>URI</Label>
              <Input
                value={testUri}
                onChange={(e) => setTestUri(e.target.value)}
                placeholder="/"
              />
            </div>
          </div>
          <Button onClick={handleTestRule} disabled={testRule.isPending || !testRuleInput.trim()}>
            <FlaskConical className="h-4 w-4 mr-1" />
            Test Rule
          </Button>
          {testResult && (
            <div className="rounded-md bg-muted p-3 font-mono text-xs">
              {testResult}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
