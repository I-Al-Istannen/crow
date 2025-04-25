import { z } from 'zod'

export const RunnerIdSchema = z.string()
export const TaskIdSchema = z.string()
export const TeamIdSchema = z.string()
export const TeamIntegrationTokenSchema = z.string()
export const TestIdSchema = z.string()
export const UserIdSchema = z.string()

export const AbortedExecutionSchema = z.object({
  stdout: z.string(),
  stderr: z.string(),
  runtime: z.number().describe('duration in ms'),
})

export const ExecutingTestSchema = z.object({
  testId: TestIdSchema,
  status: z.union([z.literal('Queued'), z.literal('Started')]),
})

export const ExecutionExitStatusSchema = z.union([
  z.literal('Aborted'),
  z.literal('Error'),
  z.literal('Failure'),
  z.literal('Success'),
  z.literal('Timeout'),
])

// Out of order due to dependencies
export const InternalErrorSchema = z.object({
  message: z.string(),
  runtime: z.number().describe('duration in ms'),
})

// Out of order due to dependencies
export const FinishedExecutionSchema = z.object({
  stdout: z.string(),
  stderr: z.string(),
  runtime: z.number().describe('duration in ms'),
  exitStatus: z.number().nullable(),
})

export const ExecutionOutputSchema = z.discriminatedUnion('type', [
  z.object({ type: z.literal('Aborted') }).merge(AbortedExecutionSchema),
  z.object({ type: z.literal('Error') }).merge(InternalErrorSchema),
  z.object({ type: z.literal('Failure') }).merge(FinishedExecutionSchema),
  z.object({ type: z.literal('Success') }).merge(FinishedExecutionSchema),
  z.object({ type: z.literal('Timeout') }).merge(FinishedExecutionSchema),
])

// Out of order due to dependencies
export const FinishedTaskInfoSchema = z.object({
  taskId: TaskIdSchema,
  start: z.number().transform((ms) => new Date(ms)),
  end: z.number().transform((ms) => new Date(ms)),
  teamId: TeamIdSchema,
  revisionId: z.string(),
  commitMessage: z.string(),
})

export const TestExecutionOutputSchema = z.discriminatedUnion('type', [
  z.object({
    type: z.literal('BinaryFailed'),
    compilerOutput: ExecutionOutputSchema,
    binaryOutput: ExecutionOutputSchema,
  }),
  z.object({ type: z.literal('CompilerFailed'), compilerOutput: ExecutionOutputSchema }),
  z.object({ type: z.literal('Error'), outputSoFar: ExecutionOutputSchema }),
  z.object({
    type: z.literal('Success'),
    compilerOutput: ExecutionOutputSchema,
    binaryOutput: ExecutionOutputSchema.nullable(),
  }),
])

// Out of order due to dependencies
export const FinishedTestSchema = z.object({
  testId: z.string(),
  output: TestExecutionOutputSchema,
})

export const FinishedCompilerTaskSchema = z.discriminatedUnion('type', [
  z.object({
    type: z.literal('BuildFailed'),
    info: FinishedTaskInfoSchema,
    buildOutput: ExecutionOutputSchema,
  }),
  z.object({
    type: z.literal('RanTests'),
    info: FinishedTaskInfoSchema,
    buildOutput: FinishedExecutionSchema,
    tests: z.array(FinishedTestSchema),
  }),
])

// Out of order due to dependencies
export const FinishedTestSummarySchema = z.object({
  testId: TestIdSchema,
  output: ExecutionExitStatusSchema,
})

export const FinishedCompilerTaskSummarySchema = z.discriminatedUnion('type', [
  z.object({
    type: z.literal('BuildFailed'),
    info: FinishedTaskInfoSchema,
    status: ExecutionExitStatusSchema,
  }),
  z.object({
    type: z.literal('RanTests'),
    info: FinishedTaskInfoSchema,
    tests: z.array(FinishedTestSummarySchema),
  }),
])

// Out of order due to dependencies
export const GithubIntegrationInfoResponse = z.object({
  url: z.string().url('Not a valid url'),
})

export const IntegrationInfoResponseSchema = z.object({
  token: TeamIntegrationTokenSchema,
  github: GithubIntegrationInfoResponse,
})

export const RepoSchema = z.object({
  team: TeamIdSchema,
  url: z.string().url('The repo URL is invalid'),
})

export const RequestRevisionSchema = z.object({
  taskId: TaskIdSchema,
})

// Out of order due to dependencies
export const WorkItemSchema = z.object({
  id: TaskIdSchema,
  team: TeamIdSchema,
  revision: z.string(),
  commitMessage: z.string(),
  insertTime: z.number().transform((ms) => new Date(ms)),
})

export const RunnerWorkingOnSchema = z.discriminatedUnion('type', [
  z.object({ type: z.literal('TestTasting') }),
  z.object({ type: z.literal('Testing') }).merge(WorkItemSchema),
])

// Out of order due to dependencies
export const RunnerSchema = z.object({
  id: RunnerIdSchema,
  info: z.string(),
  workingOn: RunnerWorkingOnSchema.nullish(),
  lastSeen: z.number().transform((ms) => new Date(ms)),
  testTaster: z.boolean(),
})

export const QueueResponseSchema = z.object({
  queue: z.array(WorkItemSchema),
  runners: z.array(RunnerSchema),
})

// Out of order due to dependencies
export const UserSchema = z.object({
  id: UserIdSchema,
  displayName: z.string(),
  team: TeamIdSchema.nullable(),
})

// Out of order due to dependencies
export const TeamSchema = z.object({
  id: TeamIdSchema,
  displayName: z.string(),
})

export const RunnerUpdateSchema = z.discriminatedUnion('type', [
  z.object({ type: z.literal('AllTests'), tests: z.array(TestIdSchema) }),
  z.object({ type: z.literal('StartedBuild') }),
  z.object({ type: z.literal('FinishedBuild'), result: FinishedExecutionSchema }),
  z.object({ type: z.literal('StartedTest'), testId: TestIdSchema }),
  z.object({ type: z.literal('FinishedTest'), result: FinishedTestSchema }),
  z.object({ type: z.literal('Done') }),
])

export const RunnerUpdateMessageSchema = z.object({
  update: RunnerUpdateSchema,
  time: z.number().transform((ms) => new Date(ms)),
})

export const ShowMyselfResponseSchema = z.object({
  user: UserSchema,
  team: TeamSchema.nullable(),
})

export const CrashSignalSchema = z.union([
  z.literal('SegmentationFault'),
  z.literal('FloatingPointException'),
])

export const CompilerFailReasonSchema = z.union([
  z.literal('Parsing'),
  z.literal('SemanticAnalysis'),
])

export const TestModifierSchema = z.discriminatedUnion('type', [
  z.object({ type: z.literal('ExpectedOutput'), output: z.string() }),
  z.object({ type: z.literal('ProgramArgument'), arg: z.string() }),
  z.object({ type: z.literal('ProgramArgumentFile'), contents: z.string() }),
  z.object({ type: z.literal('ProgramInput'), input: z.string() }),
  z.object({ type: z.literal('ShouldCrash'), signal: CrashSignalSchema }),
  z.object({ type: z.literal('ShouldFail'), reason: CompilerFailReasonSchema }),
  z.object({ type: z.literal('ShouldSucceed') }),
])

export const TestSchema = z.object({
  id: TestIdSchema,
  owner: TeamIdSchema,
  category: z.string(),
  compilerModifiers: z.array(TestModifierSchema),
  binaryModifiers: z.array(TestModifierSchema),
  adminAuthored: z.boolean(),
})

export const TestTastingResultSchema = z.discriminatedUnion('type', [
  z.object({ type: z.literal('Success') }),
  z.object({ type: z.literal('Failure'), output: TestExecutionOutputSchema }),
])

export const TestWithTestTastingSchema = TestSchema.merge(
  z.object({
    testTastingResult: TestTastingResultSchema.nullable(),
  }),
)

export const SetTestResponseSchema = z.discriminatedUnion('type', [
  z.object({ type: z.literal('TastingFailed'), output: TestExecutionOutputSchema }),
  z.object({ type: z.literal('TestAdded') }).merge(TestSchema),
])

export const TeamInfoSchema = z.object({
  team: TeamSchema,
  members: z.array(UserSchema),
})

export const TestSummarySchema = z.object({
  id: TestIdSchema,
  creatorId: TeamIdSchema,
  creatorName: z.string(),
  adminAuthored: z.boolean(),
  category: z.string(),
  testTasteSuccess: z.boolean().nullable(),
})

export const ListTestResponseSchema = z.object({
  tests: z.array(TestSummarySchema),
  categories: z.array(z.string()),
})

export type AbortedExecution = z.infer<typeof AbortedExecutionSchema>
export type ExecutingTest = z.infer<typeof ExecutingTestSchema>
export type ExecutionExitStatus = z.infer<typeof ExecutionExitStatusSchema>
export type ExecutionOutput = z.infer<typeof ExecutionOutputSchema>
export type FinishedCompilerTask = z.infer<typeof FinishedCompilerTaskSchema>
export type FinishedCompilerTaskSummary = z.infer<typeof FinishedCompilerTaskSummarySchema>
export type FinishedExecution = z.infer<typeof FinishedExecutionSchema>
export type FinishedTaskInfo = z.infer<typeof FinishedTaskInfoSchema>
export type FinishedTest = z.infer<typeof FinishedTestSchema>
export type FinishedTestSummary = z.infer<typeof FinishedTestSummarySchema>
export type GithubIntegrationInfoResponse = z.infer<typeof GithubIntegrationInfoResponse>
export type IntegrationInfoResponse = z.infer<typeof IntegrationInfoResponseSchema>
export type InternalError = z.infer<typeof InternalErrorSchema>
export type ListTestResponse = z.infer<typeof ListTestResponseSchema>
export type QueueResponse = z.infer<typeof QueueResponseSchema>
export type Repo = z.infer<typeof RepoSchema>
export type RequestRevision = z.infer<typeof RequestRevisionSchema>
export type Runner = z.infer<typeof RunnerSchema>
export type RunnerUpdate = z.infer<typeof RunnerUpdateSchema>
export type RunnerUpdateMessage = z.infer<typeof RunnerUpdateMessageSchema>
export type ShowMyselfResponse = z.infer<typeof ShowMyselfResponseSchema>
export type SetTestResponse = z.infer<typeof SetTestResponseSchema>
export type TaskId = z.infer<typeof TaskIdSchema>
export type Team = z.infer<typeof TeamSchema>
export type TeamId = z.infer<typeof TeamIdSchema>
export type TeamInfo = z.infer<typeof TeamInfoSchema>
export type TeamIntegrationToken = z.infer<typeof TeamIntegrationTokenSchema>
export type Test = z.infer<typeof TestSchema>
export type TestExecutionOutput = z.infer<typeof TestExecutionOutputSchema>
export type CrashSignal = z.infer<typeof CrashSignalSchema>
export type CompilerFailReason = z.infer<typeof CompilerFailReasonSchema>
export type TestModifier = z.infer<typeof TestModifierSchema>
export type TestWithTestTasting = z.infer<typeof TestWithTestTastingSchema>
export type TestId = z.infer<typeof TestIdSchema>
export type TestSummary = z.infer<typeof TestSummarySchema>
export type User = z.infer<typeof UserSchema>
export type UserId = z.infer<typeof UserIdSchema>
export type RunnerWorkingOn = z.infer<typeof RunnerWorkingOnSchema>
export type WorkItem = z.infer<typeof WorkItemSchema>

export function toExecutionStatus(output: TestExecutionOutput): ExecutionExitStatus {
  switch (output.type) {
    case 'CompilerFailed':
      return output.compilerOutput.type
    case 'BinaryFailed':
      return output.binaryOutput.type
    case 'Error':
      return output.outputSoFar.type
    case 'Success':
      return 'Success'
  }
}

export function toCompilerOutput(output: TestExecutionOutput): ExecutionOutput | undefined {
  switch (output.type) {
    case 'CompilerFailed':
      return output.compilerOutput
    case 'BinaryFailed':
      return output.compilerOutput
    case 'Error':
      return undefined
    case 'Success':
      return output.compilerOutput
  }
}

export function toBinaryOutput(output: TestExecutionOutput): ExecutionOutput | undefined {
  switch (output.type) {
    case 'CompilerFailed':
      return undefined
    case 'BinaryFailed':
      return output.binaryOutput
    case 'Error':
      return output.outputSoFar
    case 'Success':
      return output.binaryOutput || undefined
  }
}
