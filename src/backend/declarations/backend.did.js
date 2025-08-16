export const idlFactory = ({ IDL }) => {
  const Error = IDL.Variant({
    'CanisterError' : IDL.Record({ 'message' : IDL.Text }),
    'InvalidCanister' : IDL.Null,
  });
  const Result = IDL.Variant({ 'Ok' : IDL.Text, 'Err' : Error });
  const Person = IDL.Record({ 'age' : IDL.Nat64, 'name' : IDL.Text });
  const QueryParams = IDL.Record({ 'offset' : IDL.Nat64, 'limit' : IDL.Nat64 });
  const FilterParams = IDL.Record({ 'name' : IDL.Text });
  const UpdateParams = IDL.Record({ 'id' : IDL.Nat64, 'name' : IDL.Text });
  return IDL.Service({
    'create' : IDL.Func([], [Result], []),
    'delete' : IDL.Func([IDL.Nat64], [Result], []),
    'insert' : IDL.Func([Person], [Result], []),
    'query' : IDL.Func([QueryParams], [Result], ['query']),
    'query_filter' : IDL.Func([FilterParams], [Result], ['query']),
    'update' : IDL.Func([UpdateParams], [Result], []),
  });
};
export const init = ({ IDL }) => { return []; };
