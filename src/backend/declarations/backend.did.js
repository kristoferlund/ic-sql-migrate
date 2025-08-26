export const idlFactory = ({ IDL }) => {
  const PersonInput = IDL.Record({ 'age' : IDL.Nat32, 'name' : IDL.Text });
  const Person = IDL.Record({
    'id' : IDL.Nat32,
    'age' : IDL.Nat32,
    'name' : IDL.Text,
  });
  const Result = IDL.Variant({ 'Ok' : Person, 'Err' : IDL.Text });
  const QueryParams = IDL.Record({ 'offset' : IDL.Nat32, 'limit' : IDL.Nat32 });
  const Result_1 = IDL.Variant({ 'Ok' : IDL.Vec(Person), 'Err' : IDL.Text });
  const UpdateParams = IDL.Record({ 'id' : IDL.Nat32, 'name' : IDL.Text });
  return IDL.Service({
    'person_create' : IDL.Func([PersonInput], [Result], []),
    'person_delete' : IDL.Func([IDL.Nat32], [Result], []),
    'person_query' : IDL.Func([QueryParams], [Result_1], ['query']),
    'person_update' : IDL.Func([UpdateParams], [Result], []),
  });
};
export const init = ({ IDL }) => { return []; };
